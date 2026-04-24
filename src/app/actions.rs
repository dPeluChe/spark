//! Action dispatch: fire background tasks based on keyboard/message actions.
//!
//! Returns true to signal "quit the event loop".

use super::spawn::{spawn_remote_checks, spawn_update_task};
use crate::scanner;
use crate::tui::model::*;
use crate::tui::update::Action;
use crate::updater::detector::Detector;
use std::sync::Arc;
use tokio::sync::mpsc;

/// Dispatch an Action produced by a key handler. Returns true if the
/// application should quit.
pub fn dispatch_action(
    action: Action,
    app: &mut App,
    tx: &mpsc::UnboundedSender<AppMessage>,
    detector: &Arc<Detector>,
) -> bool {
    match action {
        Action::Quit => return true,
        Action::StartUpdate(index) => {
            let tool = app.updater.items[index].tool.clone();
            spawn_update_task(index, tool, detector.clone(), tx.clone());
        }
        Action::StartVersionChecks => {
            spawn_remote_checks(&app.updater, detector.clone(), tx.clone());
        }
        Action::DiscoverDirs => spawn_discover_dirs(tx.clone()),
        Action::StartScan(dirs) => start_scan(app, dirs, tx.clone()),
        Action::CleanArtifacts(paths) => {
            start_clean_artifacts(paths, app.config.use_trash, tx.clone())
        }
        Action::TrashRepo(path) => start_trash_repo(path, app.config.use_trash, tx.clone()),
        Action::ScanPorts => start_scan_ports(app, tx.clone()),
        Action::ListManagedRepos => start_list_managed_repos(app, tx.clone()),
        Action::CheckRepoStatuses => start_check_repo_statuses(app, tx.clone()),
        Action::PullRepos(indices) => start_pull_repos(app, indices, tx.clone()),
        Action::CloneRepo(url) => start_clone_repo(app, url, tx.clone()),
        Action::ScanSystem => start_scan_system(app, tx.clone()),
        Action::CleanSystemItem(index) => start_clean_system_item(app, index, tx.clone()),
        Action::CleanSystemItems(indices) => start_clean_system_items(app, indices, tx.clone()),
        Action::StartAudit(path) => start_audit(app, path, tx.clone()),
        Action::LoadContainerChildren(path) => start_load_container_children(path, tx.clone()),
        Action::OpenDir(path) => open_dir(app, path),
        Action::KillProcesses(pids) => start_kill_processes(pids, tx.clone()),
    }
    false
}

/// Dispatch an Action produced by a message handler (only a small subset of
/// actions flow through here). Returns true if the application should quit.
pub fn dispatch_message_action(
    action: Action,
    app: &mut App,
    tx: &mpsc::UnboundedSender<AppMessage>,
    detector: &Arc<Detector>,
) -> bool {
    match action {
        Action::StartVersionChecks => {
            spawn_remote_checks(&app.updater, detector.clone(), tx.clone());
        }
        Action::StartUpdate(index) => {
            let tool = app.updater.items[index].tool.clone();
            spawn_update_task(index, tool, detector.clone(), tx.clone());
        }
        Action::CheckRepoStatuses => start_check_repo_statuses(app, tx.clone()),
        Action::Quit => return true,
        _ => {}
    }
    false
}

fn spawn_discover_dirs(tx: mpsc::UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        let dirs = scanner::repo_scanner::discover_project_dirs();
        let _ = tx.send(AppMessage::DiscoveredDirs { dirs });
    });
}

fn start_scan(app: &mut App, dirs: Vec<std::path::PathBuf>, tx: mpsc::UnboundedSender<AppMessage>) {
    let max_depth = app.config.max_scan_depth;
    let count = dirs.len();
    let first = dirs
        .first()
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
            mpsc::unbounded_channel::<scanner::repo_scanner::ScanProgressMsg>();
        let tx_forward = tx.clone();

        let progress_forwarder = tokio::spawn(async move {
            while let Some(msg) = progress_rx.recv().await {
                let _ = tx_forward.send(AppMessage::ScanProgress {
                    repos_found: msg.repos_found,
                    dirs_scanned: msg.dirs_scanned,
                    current_dir: msg.current_dir,
                });
            }
        });

        let repos = tokio::task::spawn_blocking(move || {
            let rt = tokio::runtime::Handle::current();
            rt.block_on(scanner::repo_scanner::scan_directories(
                &dirs,
                max_depth,
                progress_tx,
            ))
        })
        .await
        .unwrap_or_default();

        let _ = progress_forwarder.await;
        let _ = tx.send(AppMessage::ScanComplete { repos });
    });
}

fn start_clean_artifacts(
    paths: Vec<std::path::PathBuf>,
    use_trash: bool,
    tx: mpsc::UnboundedSender<AppMessage>,
) {
    tokio::spawn(async move {
        let action = scanner::cleaner::CleanAction::DeleteArtifacts(paths);
        let result = scanner::cleaner::execute_clean(&action, use_trash);
        let _ = tx.send(AppMessage::CleanResult {
            index: 0,
            bytes_recovered: result.bytes_recovered,
            success: result.success,
            error: result.error,
        });
        let _ = tx.send(AppMessage::CleanAllComplete);
    });
}

fn start_trash_repo(
    path: std::path::PathBuf,
    use_trash: bool,
    tx: mpsc::UnboundedSender<AppMessage>,
) {
    tokio::spawn(async move {
        let action = scanner::cleaner::CleanAction::TrashRepo(path);
        let result = scanner::cleaner::execute_clean(&action, use_trash);
        let _ = tx.send(AppMessage::CleanResult {
            index: 0,
            bytes_recovered: result.bytes_recovered,
            success: result.success,
            error: result.error,
        });
        let _ = tx.send(AppMessage::CleanAllComplete);
    });
}

fn start_scan_ports(app: &mut App, tx: mpsc::UnboundedSender<AppMessage>) {
    app.port_scanner.scanning = true;
    tokio::spawn(async move {
        let ports = tokio::task::spawn_blocking(scanner::port_scanner::scan_ports)
            .await
            .unwrap_or_default();
        let _ = tx.send(AppMessage::PortScanResult { ports });
    });
}

fn start_list_managed_repos(app: &App, tx: mpsc::UnboundedSender<AppMessage>) {
    let root = app.repo_manager.root.clone();
    tokio::spawn(async move {
        let repos =
            tokio::task::spawn_blocking(move || scanner::repo_manager::list_managed_repos(&root))
                .await
                .unwrap_or_default();
        let _ = tx.send(AppMessage::RepoListResult { repos });
    });
}

fn start_check_repo_statuses(app: &mut App, tx: mpsc::UnboundedSender<AppMessage>) {
    let cache = scanner::repo_manager::load_status_cache();
    let mut uncached: Vec<(usize, std::path::PathBuf)> = Vec::new();

    for (i, repo) in app.repo_manager.repos.iter_mut().enumerate() {
        let key = repo.path.display().to_string();
        if let Some((status_str, ts)) = cache.get(&key) {
            if scanner::repo_manager::is_cache_valid(*ts) {
                repo.status = scanner::repo_manager::string_to_status(status_str);
                continue;
            }
        }
        uncached.push((i, repo.path.clone()));
    }

    if uncached.is_empty() {
        return;
    }

    tokio::spawn(async move {
        for (i, path) in uncached {
            let p = path.clone();
            let status =
                tokio::task::spawn_blocking(move || scanner::repo_manager::check_repo_status(&p))
                    .await
                    .unwrap_or(scanner::repo_manager::RepoStatus::Error(
                        "Task failed".into(),
                    ));
            scanner::repo_manager::save_status_to_cache(
                &path.display().to_string(),
                &scanner::repo_manager::status_to_string(&status),
            );
            let _ = tx.send(AppMessage::RepoStatusResult { index: i, status });
        }
    });
}

fn start_pull_repos(app: &App, indices: Vec<usize>, tx: mpsc::UnboundedSender<AppMessage>) {
    for idx in indices {
        if let Some(repo) = app.repo_manager.repos.get(idx) {
            let tx_clone = tx.clone();
            let path = repo.path.clone();
            tokio::spawn(async move {
                let result =
                    tokio::task::spawn_blocking(move || scanner::repo_manager::pull_repo(&path))
                        .await;
                let (success, message) = match result {
                    Ok(Ok(msg)) => (true, msg),
                    Ok(Err(e)) => (false, e),
                    Err(e) => (false, e.to_string()),
                };
                let _ = tx_clone.send(AppMessage::RepoPullResult {
                    index: idx,
                    success,
                    message,
                });
            });
        }
    }
}

fn start_clone_repo(app: &App, url: String, tx: mpsc::UnboundedSender<AppMessage>) {
    let root = app.repo_manager.root.clone();
    tokio::spawn(async move {
        let result =
            tokio::task::spawn_blocking(move || scanner::repo_manager::clone_repo(&url, &root))
                .await;
        let (success, message, clone_path) = match result {
            Ok(Ok(path)) => {
                let p = path.display().to_string();
                (true, format!("Cloned to {}", p), Some(p))
            }
            Ok(Err(e)) => (false, e, None),
            Err(e) => (false, e.to_string(), None),
        };
        let _ = tx.send(AppMessage::CloneResult {
            success,
            message,
            clone_path,
        });
    });
}

fn start_scan_system(app: &mut App, tx: mpsc::UnboundedSender<AppMessage>) {
    app.system_cleaner.scanning = true;
    tokio::spawn(async move {
        let items = tokio::task::spawn_blocking(scanner::system_cleaner::scan_system)
            .await
            .unwrap_or_default();
        let _ = tx.send(AppMessage::SystemScanResult { items });
    });
}

fn start_clean_system_item(app: &App, index: usize, tx: mpsc::UnboundedSender<AppMessage>) {
    let item = match app.system_cleaner.items.get(index).cloned() {
        Some(i) => i,
        None => return,
    };
    let is_dry_run = app.dry_run;
    tokio::spawn(async move {
        let result = tokio::task::spawn_blocking(move || {
            scanner::system_cleaner::execute_clean(&item, is_dry_run)
        })
        .await
        .unwrap_or(Err("Task failed".into()));
        let (success, recovered, error) = match result {
            Ok(r) => (true, r, None),
            Err(e) => (false, 0, Some(e)),
        };
        let _ = tx.send(AppMessage::SystemCleanItemResult {
            index,
            recovered,
            success,
            error,
        });
    });
}

fn start_clean_system_items(app: &App, indices: Vec<usize>, tx: mpsc::UnboundedSender<AppMessage>) {
    let is_dry_run = app.dry_run;
    for index in indices {
        if let Some(item) = app.system_cleaner.items.get(index).cloned() {
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let result = tokio::task::spawn_blocking(move || {
                    scanner::system_cleaner::execute_clean(&item, is_dry_run)
                })
                .await
                .unwrap_or(Err("Task failed".into()));
                let (success, recovered, error) = match result {
                    Ok(r) => (true, r, None),
                    Err(e) => (false, 0, Some(e)),
                };
                let _ = tx_clone.send(AppMessage::SystemCleanItemResult {
                    index,
                    recovered,
                    success,
                    error,
                });
            });
        }
    }
}

fn start_audit(app: &mut App, path: std::path::PathBuf, tx: mpsc::UnboundedSender<AppMessage>) {
    app.audit.scan_path = Some(path.clone());
    app.audit.scanning = true;
    app.scanner.state = ScannerState::SecretAuditScanning;
    tokio::spawn(async move {
        let path2 = path.clone();
        let results =
            tokio::task::spawn_blocking(move || scanner::secret_scanner::scan_directory(&path2))
                .await
                .unwrap_or_default();
        let dep_vulns = {
            let path3 = path.clone();
            let deps = tokio::task::spawn_blocking(move || {
                scanner::dep_scanner::parse_dependencies(&path3)
            })
            .await
            .unwrap_or_default();
            if !deps.is_empty() {
                scanner::dep_scanner::check_vulnerabilities(&deps)
                    .await
                    .vulnerabilities
            } else {
                Vec::new()
            }
        };
        let _ = tx.send(AppMessage::AuditScanResult { results, dep_vulns });
    });
}

fn start_load_container_children(path: std::path::PathBuf, tx: mpsc::UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        let children = tokio::task::spawn_blocking(move || {
            scanner::repo_scanner::scan_container_children(&path)
        })
        .await
        .unwrap_or_default();
        let _ = tx.send(AppMessage::ContainerChildrenResult { children });
    });
}

fn open_dir(app: &mut App, path: std::path::PathBuf) {
    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());
    match std::process::Command::new(cmd).arg(&path).spawn() {
        Ok(_) => app.show_toast(format!("Opened {}", name), false),
        Err(e) => app.show_toast(format!("Failed to open: {}", e), true),
    }
}

fn start_kill_processes(pids: Vec<u32>, tx: mpsc::UnboundedSender<AppMessage>) {
    tokio::spawn(async move {
        for pid in pids {
            let result = scanner::port_scanner::kill_process(pid);
            let (success, error) = match result {
                Ok(_) => (true, None),
                Err(e) => (false, Some(e)),
            };
            let _ = tx.send(AppMessage::KillResult {
                pid,
                success,
                error,
            });
        }
    });
}
