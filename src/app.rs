use std::sync::Arc;
use std::time::Duration;

use crossterm::event::{self, Event};
use ratatui::prelude::*;
use tokio::sync::mpsc;

use crate::config::SparkConfig;
use crate::tui::model::*;
use crate::tui::update::{self, Action};
use crate::tui::view;
use crate::updater::detector::Detector;

/// Run the application event loop
pub async fn run(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    config: SparkConfig,
    scan_only: bool,
    _update_only: bool,
    dry_run: bool,
) -> color_eyre::Result<()> {
    let mut app = App::new(config.clone());
    app.dry_run = dry_run;

    // Start in scanner mode if --scan-only
    if scan_only {
        app.mode = AppMode::Scanner;
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<AppMessage>();
    let detector = Arc::new(Detector::new());

    // Start initial version checks and cache warmup
    spawn_version_checks(&app.updater, detector.clone(), tx.clone());
    spawn_warmup(detector.clone(), tx.clone());

    // Splash timer
    let splash_tx = tx.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(2)).await;
        // Splash auto-advance is handled by tick
        let _ = splash_tx; // keep alive
    });

    let tick_rate = Duration::from_millis(100);

    loop {
        // Draw
        terminal.draw(|frame| view::draw(frame, &app))?;

        // Poll events with tick rate
        let has_event = tokio::task::spawn_blocking(move || {
            event::poll(tick_rate).unwrap_or(false)
        })
        .await
        .unwrap_or(false);

        if has_event {
            if let Ok(Event::Key(key)) = event::read() {
                if let Some(action) = update::handle_key(&mut app, key) {
                    match action {
                        Action::Quit => break,
                        Action::StartUpdate(index) => {
                            let tool = app.updater.items[index].tool.clone();
                            spawn_update_task(index, tool, detector.clone(), tx.clone());
                        }
                        Action::StartVersionChecks => {
                            // Remote version checks after warmup
                            spawn_remote_checks(&app.updater, detector.clone(), tx.clone());
                        }
                        Action::DiscoverDirs => {
                            let tx2 = tx.clone();
                            tokio::spawn(async move {
                                let dirs =
                                    crate::scanner::repo_scanner::discover_project_dirs();
                                let _ = tx2.send(AppMessage::DiscoveredDirs { dirs });
                            });
                        }
                        Action::StartScan(dirs) => {
                            let tx2 = tx.clone();
                            let max_depth = config.max_scan_depth;
                            tokio::spawn(async move {
                                let (progress_tx, mut progress_rx) =
                                    mpsc::unbounded_channel::<crate::scanner::repo_scanner::ScanProgressMsg>();
                                let tx3 = tx2.clone();

                                // Forward scan progress
                                let progress_forwarder = tokio::spawn(async move {
                                    while let Some(msg) = progress_rx.recv().await {
                                        let _ = tx3.send(AppMessage::ScanProgress {
                                            repos_found: msg.repos_found,
                                            dirs_scanned: msg.dirs_scanned,
                                            current_dir: msg.current_dir,
                                        });
                                    }
                                });

                                let repos =
                                    crate::scanner::repo_scanner::scan_directories(
                                        &dirs, max_depth, progress_tx,
                                    )
                                    .await;

                                let _ = progress_forwarder.await;
                                let _ = tx2.send(AppMessage::ScanComplete { repos });
                            });
                        }
                        Action::CleanArtifacts(paths) => {
                            let tx2 = tx.clone();
                            let use_trash = config.use_trash;
                            tokio::spawn(async move {
                                let action =
                                    crate::scanner::cleaner::CleanAction::DeleteArtifacts(paths);
                                let result =
                                    crate::scanner::cleaner::execute_clean(&action, use_trash);
                                let _ = tx2.send(AppMessage::CleanResult {
                                    index: 0,
                                    bytes_recovered: result.bytes_recovered,
                                    success: result.success,
                                    error: result.error,
                                });
                                let _ = tx2.send(AppMessage::CleanAllComplete);
                            });
                        }
                        Action::TrashRepo(path) => {
                            let tx2 = tx.clone();
                            let use_trash = config.use_trash;
                            tokio::spawn(async move {
                                let action =
                                    crate::scanner::cleaner::CleanAction::TrashRepo(path);
                                let result =
                                    crate::scanner::cleaner::execute_clean(&action, use_trash);
                                let _ = tx2.send(AppMessage::CleanResult {
                                    index: 0,
                                    bytes_recovered: result.bytes_recovered,
                                    success: result.success,
                                    error: result.error,
                                });
                                let _ = tx2.send(AppMessage::CleanAllComplete);
                            });
                        }
                        Action::ScanPorts => {
                            let tx2 = tx.clone();
                            tokio::spawn(async move {
                                let ports =
                                    tokio::task::spawn_blocking(crate::scanner::port_scanner::scan_ports)
                                        .await
                                        .unwrap_or_default();
                                let _ = tx2.send(AppMessage::PortScanResult { ports });
                            });
                        }
                        Action::KillProcesses(pids) => {
                            let tx2 = tx.clone();
                            tokio::spawn(async move {
                                for pid in pids {
                                    let result = crate::scanner::port_scanner::kill_process(pid);
                                    let (success, error) = match result {
                                        Ok(_) => (true, None),
                                        Err(e) => (false, Some(e)),
                                    };
                                    let _ = tx2.send(AppMessage::KillResult {
                                        pid,
                                        success,
                                        error,
                                    });
                                }
                            });
                        }
                    }
                }
            } else if let Ok(Event::Resize(w, h)) = event::read() {
                app.width = w;
                app.height = h;
            }
        }

        // Process background messages
        while let Ok(msg) = rx.try_recv() {
            if let Some(action) = update::handle_message(&mut app, msg) {
                match action {
                    Action::StartVersionChecks => {
                        spawn_remote_checks(&app.updater, detector.clone(), tx.clone());
                    }
                    Action::StartUpdate(index) => {
                        let tool = app.updater.items[index].tool.clone();
                        spawn_update_task(index, tool, detector.clone(), tx.clone());
                    }
                    Action::Quit => break,
                    _ => {}
                }
            }
        }

        // Tick animation
        app.tick_count = app.tick_count.wrapping_add(1);
        if app.updater.state == UpdaterState::Splash
            || app.updater.state == UpdaterState::Updating
        {
            app.updater.splash_frame = app.tick_count;
        }

        // Auto-advance splash after ~20 ticks (2 seconds at 100ms)
        if app.updater.state == UpdaterState::Splash && app.tick_count >= 20 {
            app.updater.state = UpdaterState::Main;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}

fn spawn_version_checks(
    updater: &UpdaterModel,
    detector: Arc<Detector>,
    tx: mpsc::UnboundedSender<AppMessage>,
) {
    for (i, item) in updater.items.iter().enumerate() {
        let tool = item.tool.clone();
        let detector = detector.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            let local = detector.get_local_version(&tool).await;

            let status = if local == "MISSING" {
                crate::core::types::ToolStatus::Missing
            } else {
                crate::core::types::ToolStatus::Installed
            };

            let message = if local == "MISSING" {
                "Not installed".into()
            } else {
                String::new()
            };

            let _ = tx.send(AppMessage::CheckResult {
                index: i,
                local_version: local,
                remote_version: "...".into(),
                status,
                message,
            });
        });
    }
}

fn spawn_remote_checks(
    updater: &UpdaterModel,
    detector: Arc<Detector>,
    tx: mpsc::UnboundedSender<AppMessage>,
) {
    for (i, item) in updater.items.iter().enumerate() {
        let tool = item.tool.clone();
        let local = item.local_version.clone();
        let detector = detector.clone();
        let tx = tx.clone();
        tokio::spawn(async move {
            let remote = detector.get_remote_version(&tool, &local).await;

            let status = if local == "MISSING" {
                crate::core::types::ToolStatus::Missing
            } else if remote != "Unknown"
                && remote != "Checking..."
                && remote != local
            {
                crate::core::types::ToolStatus::Outdated
            } else {
                crate::core::types::ToolStatus::Installed
            };

            let message = if status == crate::core::types::ToolStatus::Outdated {
                "Update available".into()
            } else {
                String::new()
            };

            let _ = tx.send(AppMessage::CheckResult {
                index: i,
                local_version: local,
                remote_version: remote,
                status,
                message,
            });
        });
    }
}

fn spawn_warmup(detector: Arc<Detector>, tx: mpsc::UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        detector.warm_up_cache().await;
        let _ = tx.send(AppMessage::WarmUpFinished);
    });
}

fn spawn_update_task(
    index: usize,
    tool: crate::core::types::Tool,
    detector: Arc<Detector>,
    tx: mpsc::UnboundedSender<AppMessage>,
) {
    tokio::spawn(async move {
        let result = crate::updater::executor::update_tool(&tool).await;
        let (success, message) = match result {
            Ok(_) => (true, format!("Updated {}", tool.name)),
            Err(e) => (false, e),
        };
        let new_version = if success {
            detector.get_local_version(&tool).await
        } else {
            String::new()
        };
        let _ = tx.send(AppMessage::UpdateResult {
            index,
            success,
            message,
            new_version,
        });
    });
}
