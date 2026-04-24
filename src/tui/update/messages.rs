//! Background task → state: drain `AppMessage`s into the app model.

use super::Action;
use crate::core::types::*;
use crate::tui::model::*;
use crate::utils::shell::debug_log;

pub fn handle_message(app: &mut App, msg: AppMessage) -> Option<Action> {
    match msg {
        AppMessage::CheckResult {
            index,
            local_version,
            remote_version,
            status,
            message,
        } => {
            let m = &mut app.updater;
            if index < m.items.len() {
                m.items[index].local_version = local_version;
                m.items[index].remote_version = remote_version;
                m.items[index].status = status;
                m.items[index].message = message;
                if m.loading_count > 0 {
                    m.loading_count -= 1;
                }
            }
        }
        AppMessage::WarmUpFinished => {
            return Some(Action::StartVersionChecks);
        }
        AppMessage::UpdateResult {
            index,
            success,
            message,
            new_version,
        } => {
            let m = &mut app.updater;
            if index < m.items.len() {
                if success {
                    m.items[index].status = ToolStatus::Updated;
                    m.items[index].message = message;
                    if !new_version.is_empty() && new_version != "MISSING" {
                        m.items[index].local_version = new_version.clone();
                        m.items[index].remote_version = new_version;
                    }
                } else {
                    m.items[index].status = ToolStatus::Failed;
                    m.items[index].message = message;
                }
                if m.updating_remaining > 0 {
                    m.updating_remaining -= 1;
                }
                m.current_update = None;

                if m.updating_remaining == 0 && m.update_queue.is_empty() {
                    m.state = UpdaterState::Summary;
                } else if let Some(next) = m.update_queue.pop_front() {
                    m.current_update = Some(next);
                    m.current_log = UpdaterModel::get_update_log_text(&m.items[next].tool);
                    return Some(Action::StartUpdate(next));
                }
            }
        }
        AppMessage::ScanProgress {
            repos_found,
            dirs_scanned,
            current_dir,
        } => {
            let s = &mut app.scanner;
            s.scan_progress_repos = repos_found;
            s.scan_progress_dirs = dirs_scanned;
            s.scan_progress_current = current_dir;
        }
        AppMessage::ScanComplete { repos } => {
            let s = &mut app.scanner;
            s.repos = repos;
            s.total_recoverable = s.repos.iter().map(|r| r.artifact_size).sum();
            s.rebuild_group_order();
            s.state = ScannerState::ScanResults;
            s.cursor = 0;
        }
        AppMessage::CleanResult {
            index,
            bytes_recovered,
            success,
            error,
        } => {
            app.scanner
                .clean_results
                .push((index, bytes_recovered, success, error));
        }
        AppMessage::CleanAllComplete => handle_clean_all_complete(app),
        AppMessage::PortScanResult { ports } => {
            app.port_scanner.ports = ports;
            app.port_scanner.cursor = 0;
            app.port_scanner.checked.clear();
            app.port_scanner.scanning = false;
            rebuild_port_display_order(&mut app.port_scanner);
        }
        AppMessage::KillResult {
            pid,
            success,
            error,
        } => handle_kill_result(app, pid, success, error),
        AppMessage::RepoListResult { repos } => {
            app.repo_manager.repos = repos;
            app.repo_manager.cursor = 0;
            app.repo_manager.checked.clear();
            return Some(Action::CheckRepoStatuses);
        }
        AppMessage::RepoStatusResult { index, status } => {
            if index < app.repo_manager.repos.len() {
                app.repo_manager.repos[index].status = status;
            }
        }
        AppMessage::RepoPullResult {
            index,
            success,
            message,
        } => handle_pull_result(app, index, success, message),
        AppMessage::CloneResult {
            success,
            message,
            clone_path,
        } => handle_clone_result(app, success, message, clone_path),
        AppMessage::SystemScanResult { items } => {
            app.system_cleaner.items = items;
            app.system_cleaner.cursor = 0;
            app.system_cleaner.checked.clear();
            app.system_cleaner.scanning = false;
            app.system_cleaner.rebuild_display_order();
        }
        AppMessage::SystemCleanItemResult {
            index,
            recovered,
            success,
            error,
        } => handle_system_clean_item_result(app, index, recovered, success, error),
        AppMessage::ContainerChildrenResult { children } => {
            app.scanner.container_children = children;
            app.scanner.container_cursor = 0;
            if app.scanner.state == ScannerState::ContainerLoading {
                app.scanner.state = ScannerState::RepoDetail;
            }
        }
        AppMessage::AuditScanResult { results, dep_vulns } => {
            app.audit.total_critical = results.iter().map(|r| r.critical_count).sum();
            app.audit.total_warning = results.iter().map(|r| r.warning_count).sum();
            app.audit.total_info = results.iter().map(|r| r.info_count).sum();
            app.audit.results = results;
            app.audit.dep_vulns = dep_vulns;
            app.audit.dep_cursor = 0;
            app.audit.scanning = false;
            app.audit.cursor = 0;
            app.scanner.state = ScannerState::SecretAudit;
        }
        AppMessage::DiscoveredDirs { dirs } => {
            debug_log(&format!("DiscoveredDirs: {} dirs found", dirs.len()));
            for d in &dirs {
                debug_log(&format!("  dir: {:?} ({} repos)", d.path, d.repo_count));
            }
            app.scanner.discovered_dirs = dirs;
            app.scanner.selected_scan_dirs.clear();
        }
    }
    None
}

fn handle_clean_all_complete(app: &mut App) {
    let recovered: u64 = app
        .scanner
        .clean_results
        .iter()
        .map(|(_, b, _, _)| *b)
        .sum();
    let ok = app
        .scanner
        .clean_results
        .iter()
        .filter(|(_, _, s, _)| *s)
        .count();
    let fail = app.scanner.clean_results.len() - ok;
    if fail == 0 {
        app.show_toast(
            format!(
                "Cleaned {} recovered",
                crate::utils::fs::format_size(recovered)
            ),
            false,
        );
    } else {
        app.show_toast(format!("Clean: {} ok, {} failed", ok, fail), true);
    }
    for repo in app.scanner.repos.iter_mut() {
        repo.artifacts.retain(|a| a.path.exists());
        repo.artifact_size = repo.artifacts.iter().map(|a| a.size).sum();
    }
    for child in app.scanner.container_children.iter_mut() {
        child.artifacts.retain(|a| a.path.exists());
        child.artifact_size = child.artifacts.iter().map(|a| a.size).sum();
    }
    app.scanner.checked.clear();
    app.scanner.clean_results.clear();
    if !app.scanner.container_children.is_empty()
        && !app.scanner.repos.is_empty()
        && app
            .scanner
            .repos
            .get(app.scanner.cursor)
            .map(|r| r.is_container)
            .unwrap_or(false)
    {
        app.scanner.state = ScannerState::ContainerChildDetail;
    } else {
        app.scanner.state = ScannerState::ScanResults;
    }
}

fn handle_kill_result(app: &mut App, pid: u32, success: bool, error: Option<String>) {
    if success {
        let name = app
            .port_scanner
            .ports
            .iter()
            .find(|p| p.pid == pid)
            .map(|p| {
                let project = p.project_dir.as_deref().unwrap_or("");
                if project.is_empty() || project == "/" {
                    format!(":{} ({})", p.port, p.process_name)
                } else {
                    let short = project.split(" (").next().unwrap_or(project);
                    format!(":{} {}", p.port, short)
                }
            })
            .unwrap_or_else(|| format!("PID {}", pid));

        app.port_scanner.ports.retain(|p| p.pid != pid);
        rebuild_port_display_order(&mut app.port_scanner);
        if app.port_scanner.cursor >= app.port_scanner.display_order.len()
            && app.port_scanner.cursor > 0
        {
            app.port_scanner.cursor -= 1;
        }
        app.show_toast(format!("Killed {}", name), false);
    }
    if let Some(err) = error {
        app.show_toast(format!("Kill failed: {}", err), true);
    }
    if app.scanner.state == ScannerState::PortKillConfirm {
        app.scanner.state = ScannerState::PortScan;
        app.port_scanner.checked.clear();
    }
}

fn handle_pull_result(app: &mut App, index: usize, success: bool, message: String) {
    if index < app.repo_manager.repos.len() {
        let name = app.repo_manager.repos[index].name.clone();
        if success {
            app.repo_manager.repos[index].status =
                crate::scanner::repo_manager::RepoStatus::UpToDate;
            app.show_toast(format!("Pulled {}", name), false);
        } else {
            app.repo_manager.repos[index].status =
                crate::scanner::repo_manager::RepoStatus::Error(message.clone());
            app.show_toast(format!("Pull failed: {} - {}", name, message), true);
        }
    }
    app.repo_manager.checked.remove(&index);
}

fn handle_clone_result(app: &mut App, success: bool, message: String, clone_path: Option<String>) {
    app.repo_manager.cloning = false;
    if !success {
        app.show_toast(format!("Clone failed: {}", message), true);
        app.repo_manager.clone_error = Some(message);
        return;
    }

    let url = app.repo_manager.clone_input.clone();
    let path = clone_path.unwrap_or_else(|| message.clone());

    let home = std::env::var("HOME").unwrap_or_default();
    let short_path = if path.starts_with(&home) {
        format!("~{}", &path[home.len()..])
    } else {
        path.clone()
    };

    let repo_name = std::path::Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let alias_cmd = format!("alias {}='cd {}'", repo_name.replace('-', "_"), short_path);

    app.repo_manager.last_clone = Some(CloneSummary {
        repo_path: path,
        repo_name,
        remote_url: url,
        alias_cmd,
        short_path,
    });

    let cloned_name = app
        .repo_manager
        .last_clone
        .as_ref()
        .map(|c| c.repo_name.clone())
        .unwrap_or_default();
    app.repo_manager.clone_input.clear();
    app.repo_manager.clone_error = None;
    app.scanner.state = ScannerState::RepoCloneSummary;
    app.show_toast(format!("Cloned {}", cloned_name), false);
}

fn handle_system_clean_item_result(
    app: &mut App,
    index: usize,
    recovered: u64,
    success: bool,
    error: Option<String>,
) {
    if success {
        let name = app
            .system_cleaner
            .items
            .get(index)
            .map(|i| i.name.clone())
            .unwrap_or_default();
        app.show_toast(
            format!(
                "Cleaned {} ({})",
                name,
                crate::utils::fs::format_size(recovered)
            ),
            false,
        );
        if index < app.system_cleaner.items.len() {
            app.system_cleaner.items.remove(index);
        }
        app.system_cleaner.rebuild_display_order();
    } else if let Some(err) = error {
        app.show_toast(format!("Clean failed: {}", err), true);
    }
    app.system_cleaner.checked.remove(&index);
}

/// Dev servers first (sorted by project name, then port), then system processes
/// (sorted by process name, then port).
fn rebuild_port_display_order(model: &mut PortScannerModel) {
    use crate::scanner::port_scanner;

    let mut dev_indices: Vec<usize> = Vec::new();
    let mut sys_indices: Vec<usize> = Vec::new();

    for (i, p) in model.ports.iter().enumerate() {
        if port_scanner::is_dev_server(p) {
            dev_indices.push(i);
        } else {
            sys_indices.push(i);
        }
    }

    dev_indices.sort_by(|&a, &b| {
        let pa = project_name(&model.ports[a].project_dir);
        let pb = project_name(&model.ports[b].project_dir);
        let has_a = !pa.is_empty();
        let has_b = !pb.is_empty();
        has_b
            .cmp(&has_a)
            .then_with(|| pa.to_lowercase().cmp(&pb.to_lowercase()))
            .then_with(|| model.ports[a].port.cmp(&model.ports[b].port))
    });

    sys_indices.sort_by(|&a, &b| {
        model.ports[a]
            .process_name
            .to_lowercase()
            .cmp(&model.ports[b].process_name.to_lowercase())
            .then_with(|| model.ports[a].port.cmp(&model.ports[b].port))
    });

    model.display_order.clear();
    model.display_order.extend(dev_indices);
    model.display_order.extend(sys_indices);
}

fn project_name(project_dir: &Option<String>) -> String {
    match project_dir {
        Some(p) if p != "/" && !p.is_empty() => {
            if let Some(paren) = p.find(" (") {
                p[..paren].to_string()
            } else {
                p.rsplit('/').next().unwrap_or(p).to_string()
            }
        }
        _ => String::new(),
    }
}
