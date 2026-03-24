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
    let mut app = App::new(config);
    app.dry_run = dry_run;

    // Skip welcome if --scan-only
    if scan_only {
        app.show_welcome = false;
        app.mode = AppMode::Scanner;
    }

    let (tx, mut rx) = mpsc::unbounded_channel::<AppMessage>();
    let detector = Arc::new(Detector::new());

    // Start initial version checks and cache warmup in background
    spawn_version_checks(&app.updater, detector.clone(), tx.clone());
    spawn_warmup(detector.clone(), tx.clone());

    // Pre-discover directories so scanner is ready
    {
        let tx2 = tx.clone();
        tokio::spawn(async move {
            let dirs = crate::scanner::repo_scanner::discover_project_dirs();
            let _ = tx2.send(AppMessage::DiscoveredDirs { dirs });
        });
    }

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
                            let max_depth = app.config.max_scan_depth;
                            let count = dirs.len();
                            let first = dirs.first()
                                .and_then(|d| d.file_name())
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let label = if count == 1 {
                                format!("Scanning {}...", first)
                            } else {
                                format!("Scanning {} dirs ({}, ...)", count, first)
                            };
                            app.show_toast(label, false);

                            tokio::spawn(async move {
                                let (progress_tx, mut progress_rx) =
                                    mpsc::unbounded_channel::<crate::scanner::repo_scanner::ScanProgressMsg>();
                                let tx3 = tx2.clone();

                                // Forward progress from blocking thread to TUI
                                let progress_forwarder = tokio::spawn(async move {
                                    while let Some(msg) = progress_rx.recv().await {
                                        let _ = tx3.send(AppMessage::ScanProgress {
                                            repos_found: msg.repos_found,
                                            dirs_scanned: msg.dirs_scanned,
                                            current_dir: msg.current_dir,
                                        });
                                    }
                                });

                                // Run scan on blocking thread so progress can flow
                                let repos = tokio::task::spawn_blocking(move || {
                                    let rt = tokio::runtime::Handle::current();
                                    rt.block_on(
                                        crate::scanner::repo_scanner::scan_directories(
                                            &dirs, max_depth, progress_tx,
                                        )
                                    )
                                }).await.unwrap_or_default();

                                let _ = progress_forwarder.await;
                                let _ = tx2.send(AppMessage::ScanComplete { repos });
                            });
                        }
                        Action::CleanArtifacts(paths) => {
                            let tx2 = tx.clone();
                            let use_trash = app.config.use_trash;
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
                            let use_trash = app.config.use_trash;
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
                            app.port_scanner.scanning = true;
                            let tx2 = tx.clone();
                            tokio::spawn(async move {
                                let ports =
                                    tokio::task::spawn_blocking(crate::scanner::port_scanner::scan_ports)
                                        .await
                                        .unwrap_or_default();
                                let _ = tx2.send(AppMessage::PortScanResult { ports });
                            });
                        }
                        Action::ListManagedRepos => {
                            let tx2 = tx.clone();
                            let root = app.repo_manager.root.clone();
                            tokio::spawn(async move {
                                let repos = tokio::task::spawn_blocking(move || {
                                    crate::scanner::repo_manager::list_managed_repos(&root)
                                })
                                .await
                                .unwrap_or_default();
                                let _ = tx2.send(AppMessage::RepoListResult { repos });
                            });
                        }
                        Action::CheckRepoStatuses => {
                            for (i, repo) in app.repo_manager.repos.iter().enumerate() {
                                let tx2 = tx.clone();
                                let path = repo.path.clone();
                                tokio::spawn(async move {
                                    let status = tokio::task::spawn_blocking(move || {
                                        crate::scanner::repo_manager::check_repo_status(&path)
                                    })
                                    .await
                                    .unwrap_or(crate::scanner::repo_manager::RepoStatus::Error(
                                        "Task failed".into(),
                                    ));
                                    let _ = tx2.send(AppMessage::RepoStatusResult { index: i, status });
                                });
                            }
                        }
                        Action::PullRepos(indices) => {
                            for idx in indices {
                                if let Some(repo) = app.repo_manager.repos.get(idx) {
                                    let tx2 = tx.clone();
                                    let path = repo.path.clone();
                                    tokio::spawn(async move {
                                        let result = tokio::task::spawn_blocking(move || {
                                            crate::scanner::repo_manager::pull_repo(&path)
                                        })
                                        .await;
                                        let (success, message) = match result {
                                            Ok(Ok(msg)) => (true, msg),
                                            Ok(Err(e)) => (false, e),
                                            Err(e) => (false, e.to_string()),
                                        };
                                        let _ = tx2.send(AppMessage::RepoPullResult {
                                            index: idx,
                                            success,
                                            message,
                                        });
                                    });
                                }
                            }
                        }
                        Action::CloneRepo(url) => {
                            let tx2 = tx.clone();
                            let root = app.repo_manager.root.clone();
                            tokio::spawn(async move {
                                let result = tokio::task::spawn_blocking(move || {
                                    crate::scanner::repo_manager::clone_repo(&url, &root)
                                })
                                .await;
                                let (success, message, clone_path) = match result {
                                    Ok(Ok(path)) => {
                                        let p = path.display().to_string();
                                        (true, format!("Cloned to {}", p), Some(p))
                                    }
                                    Ok(Err(e)) => (false, e, None),
                                    Err(e) => (false, e.to_string(), None),
                                };
                                let _ = tx2.send(AppMessage::CloneResult {
                                    success,
                                    message,
                                    clone_path,
                                });
                            });
                        }
                        Action::LoadContainerChildren(path) => {
                            let tx2 = tx.clone();
                            tokio::spawn(async move {
                                let children = tokio::task::spawn_blocking(move || {
                                    crate::scanner::repo_scanner::scan_container_children(&path)
                                }).await.unwrap_or_default();
                                let _ = tx2.send(AppMessage::ContainerChildrenResult { children });
                            });
                        }
                        Action::OpenDir(path) => {
                            let cmd = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
                            let name = path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_else(|| path.display().to_string());
                            match std::process::Command::new(cmd).arg(&path).spawn() {
                                Ok(_) => app.show_toast(format!("Opened {}", name), false),
                                Err(e) => app.show_toast(format!("Failed to open: {}", e), true),
                            }
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
        if app.updater.state == UpdaterState::Updating {
            app.updater.splash_frame = app.tick_count;
        }

        // Auto-dismiss toast after 30 ticks (3 seconds)
        if let Some(toast) = &app.toast {
            if app.tick_count.saturating_sub(toast.created_at) >= 30 {
                app.toast = None;
            }
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
