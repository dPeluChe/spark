use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::core::types::*;
use crate::tui::model::*;
use crate::tui::scanner_keys;
use crate::utils::shell::debug_log;

/// Side-effect actions dispatched from key/message handlers to the event loop
pub enum Action {
    /// Exit the application
    Quit,
    /// Trigger remote version checks after cache warmup
    StartVersionChecks,
    /// Begin updating a specific tool by index
    StartUpdate(usize),
    /// Start scanning the given directories for repos
    StartScan(Vec<std::path::PathBuf>),
    /// Discover project directories in home folder
    DiscoverDirs,
    /// Delete artifact directories (node_modules, target/, etc.)
    CleanArtifacts(Vec<std::path::PathBuf>),
    /// Move a repository to trash
    TrashRepo(std::path::PathBuf),
    /// Scan for listening ports
    ScanPorts,
    /// Kill processes by PID list
    KillProcesses(Vec<u32>),
    /// List managed repos
    ListManagedRepos,
    /// Check status of managed repos (fetch + compare)
    CheckRepoStatuses,
    /// Pull specific repos by index
    PullRepos(Vec<usize>),
    /// Clone a repo from URL into managed root
    CloneRepo(String),
    /// Open a directory in the system file manager / terminal
    OpenDir(std::path::PathBuf),
    /// Load container children in background
    LoadContainerChildren(std::path::PathBuf),
    /// Scan system for cleanable items (Docker, caches, logs)
    ScanSystem,
    /// Clean a specific system item by index
    CleanSystemItem(usize),
}

/// Handle a key event and return optional action
pub fn handle_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    // Global: Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return Some(Action::Quit);
    }

    // Welcome screen handler
    if app.show_welcome {
        return handle_welcome_key(app, key);
    }

    debug_log(&format!("KEY: {:?} | mode={:?} scanner_state={:?}", key.code, app.mode, app.scanner.state));

    // Global: Tab cycles through views (Updater -> Scanner -> Repos -> Ports -> Updater)
    // Except during text input or active operations
    let in_text_input = app.mode == AppMode::Scanner
        && matches!(app.scanner.state, ScannerState::RepoCloneInput | ScannerState::ScanAddPath);
    let in_search = app.mode == AppMode::Updater
        && app.updater.state == UpdaterState::Search;
    let in_blocking = (app.mode == AppMode::Updater && app.updater.state == UpdaterState::Updating)
        || (app.mode == AppMode::Scanner && app.scanner.state == ScannerState::Scanning)
        || (app.mode == AppMode::Scanner && app.scanner.state == ScannerState::Cleaning)
        || (app.mode == AppMode::Scanner && app.scanner.state == ScannerState::ContainerLoading);

    if key.code == KeyCode::Tab && !in_text_input && !in_search && !in_blocking {
        // Cycle: Scanner -> Repos -> Ports -> Updater -> Scanner
        // Scanner base states -> Repos
        if app.mode == AppMode::Scanner && matches!(
            app.scanner.state,
            ScannerState::ScanConfig | ScannerState::ScanResults
                | ScannerState::RepoDetail
        ) {
            app.scanner.state = ScannerState::RepoManager;
            return Some(Action::ListManagedRepos);
        }
        // Repos -> Ports
        if app.mode == AppMode::Scanner && matches!(
            app.scanner.state,
            ScannerState::RepoManager | ScannerState::RepoCloneSummary
        ) {
            app.scanner.state = ScannerState::PortScan;
            return Some(Action::ScanPorts);
        }
        // Ports -> System
        if app.mode == AppMode::Scanner && matches!(
            app.scanner.state,
            ScannerState::PortScan
        ) {
            app.scanner.state = ScannerState::SystemClean;
            return Some(Action::ScanSystem);
        }
        // System -> Updater
        if app.mode == AppMode::Scanner && matches!(
            app.scanner.state,
            ScannerState::SystemClean
        ) {
            app.mode = AppMode::Updater;
            return None;
        }
        // Updater -> Scanner (preserve ScanResults if we had them)
        if app.mode == AppMode::Updater {
            app.mode = AppMode::Scanner;
            if !app.scanner.repos.is_empty() {
                app.scanner.state = ScannerState::ScanResults;
            } else {
                app.scanner.state = ScannerState::ScanConfig;
                if app.scanner.discovered_dirs.is_empty() {
                    return Some(Action::DiscoverDirs);
                }
            }
            return None;
        }
    }

    match app.mode {
        AppMode::Updater => handle_updater_key(app, key),
        AppMode::Scanner => scanner_keys::handle_scanner_key(app, key),
    }
}

fn handle_welcome_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
            Some(Action::Quit)
        }
        KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Enter => {
            app.show_welcome = false;
            app.mode = AppMode::Scanner;
            if app.scanner.discovered_dirs.is_empty() {
                return Some(Action::DiscoverDirs);
            }
            None
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.show_welcome = false;
            app.mode = AppMode::Scanner;
            app.scanner.state = ScannerState::SystemClean;
            Some(Action::ScanSystem)
        }
        KeyCode::Char('u') | KeyCode::Char('U') => {
            app.show_welcome = false;
            app.mode = AppMode::Updater;
            None
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.show_welcome = false;
            app.mode = AppMode::Scanner;
            app.scanner.state = ScannerState::PortScan;
            Some(Action::ScanPorts)
        }
        KeyCode::Char('g') | KeyCode::Char('G') => {
            app.show_welcome = false;
            app.mode = AppMode::Scanner;
            app.scanner.state = ScannerState::RepoManager;
            Some(Action::ListManagedRepos)
        }
        _ => None,
    }
}

/// Handle messages from background tasks
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
        AppMessage::CleanAllComplete => {
            let recovered: u64 = app.scanner.clean_results.iter().map(|(_, b, _, _)| *b).sum();
            let ok = app.scanner.clean_results.iter().filter(|(_, _, s, _)| *s).count();
            let fail = app.scanner.clean_results.len() - ok;
            if fail == 0 {
                app.show_toast(format!("Cleaned {} recovered", crate::utils::fs::format_size(recovered)), false);
            } else {
                app.show_toast(format!("Clean: {} ok, {} failed", ok, fail), true);
            }
            // Refresh artifact data for cleaned repos and container children
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
            // If cleaning was triggered from container child detail, return there
            if !app.scanner.container_children.is_empty() && !app.scanner.repos.is_empty()
                && app.scanner.repos.get(app.scanner.cursor).map(|r| r.is_container).unwrap_or(false)
            {
                app.scanner.state = ScannerState::ContainerChildDetail;
            } else {
                app.scanner.state = ScannerState::ScanResults;
            }
        }
        AppMessage::PortScanResult { ports } => {
            app.port_scanner.ports = ports;
            app.port_scanner.cursor = 0;
            app.port_scanner.checked.clear();
            app.port_scanner.scanning = false;
            rebuild_port_display_order(&mut app.port_scanner);
        }
        AppMessage::KillResult { pid, success, error } => {
            if success {
                // Find name before removing
                let name = app.port_scanner.ports.iter()
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
        AppMessage::RepoListResult { repos } => {
            app.repo_manager.repos = repos;
            app.repo_manager.cursor = 0;
            app.repo_manager.checked.clear();
            // Trigger status checks
            return Some(Action::CheckRepoStatuses);
        }
        AppMessage::RepoStatusResult { index, status } => {
            if index < app.repo_manager.repos.len() {
                app.repo_manager.repos[index].status = status;
            }
        }
        AppMessage::RepoPullResult { index, success, message } => {
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
        AppMessage::CloneResult { success, message, clone_path } => {
            app.repo_manager.cloning = false;
            if success {
                let url = app.repo_manager.clone_input.clone();
                let path = clone_path.unwrap_or_else(|| message.clone());

                // Build summary with alias and agent tips
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

                let alias_cmd = format!(
                    "alias {}='cd {}'",
                    repo_name.replace('-', "_"),
                    short_path
                );

                app.repo_manager.last_clone = Some(CloneSummary {
                    repo_path: path,
                    repo_name,
                    remote_url: url,
                    alias_cmd,
                    short_path,
                });

                let cloned_name = app.repo_manager.last_clone.as_ref()
                    .map(|c| c.repo_name.clone()).unwrap_or_default();
                app.repo_manager.clone_input.clear();
                app.repo_manager.clone_error = None;
                app.scanner.state = ScannerState::RepoCloneSummary;
                app.show_toast(format!("Cloned {}", cloned_name), false);
            } else {
                app.show_toast(format!("Clone failed: {}", message), true);
                app.repo_manager.clone_error = Some(message);
            }
        }
        AppMessage::SystemScanResult { items } => {
            app.system_cleaner.items = items;
            app.system_cleaner.cursor = 0;
            app.system_cleaner.checked.clear();
            app.system_cleaner.scanning = false;
        }
        AppMessage::SystemCleanItemResult { index, recovered, success, error } => {
            if success {
                let name = app.system_cleaner.items.get(index)
                    .map(|i| i.name.clone()).unwrap_or_default();
                app.show_toast(
                    format!("Cleaned {} ({})", name, crate::utils::fs::format_size(recovered)),
                    false,
                );
                // Remove cleaned item
                if index < app.system_cleaner.items.len() {
                    app.system_cleaner.items.remove(index);
                    if app.system_cleaner.cursor > 0
                        && app.system_cleaner.cursor >= app.system_cleaner.items.len()
                    {
                        app.system_cleaner.cursor -= 1;
                    }
                }
            } else if let Some(err) = error {
                app.show_toast(format!("Clean failed: {}", err), true);
            }
            app.system_cleaner.checked.remove(&index);
        }
        AppMessage::ContainerChildrenResult { children } => {
            app.scanner.container_children = children;
            app.scanner.container_cursor = 0;
            if app.scanner.state == ScannerState::ContainerLoading {
                app.scanner.state = ScannerState::RepoDetail;
            }
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

fn handle_updater_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    let m = &mut app.updater;

    match m.state {
        UpdaterState::Search => {
            match key.code {
                KeyCode::Esc => {
                    m.state = UpdaterState::Main;
                    m.search_query.clear();
                    m.filtered_indices = None;
                }
                KeyCode::Enter => {
                    m.state = UpdaterState::Main;
                }
                KeyCode::Backspace => {
                    m.search_query.pop();
                    m.update_filter();
                }
                KeyCode::Char(c) => {
                    m.search_query.push(c);
                    m.update_filter();
                }
                _ => {}
            }
            None
        }

        UpdaterState::Preview => match key.code {
            KeyCode::Enter => {
                if m.has_critical_selected() {
                    m.state = UpdaterState::Confirm;
                } else {
                    m.state = UpdaterState::Updating;
                    return start_updates(m);
                }
                None
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                m.state = UpdaterState::Main;
                None
            }
            _ => None,
        },

        UpdaterState::Confirm => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                m.state = UpdaterState::Updating;
                start_updates(m)
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
                m.state = UpdaterState::Main;
                None
            }
            _ => None,
        },

        UpdaterState::Updating => None, // Block all input except Ctrl+C (handled globally)

        UpdaterState::Summary => {
            // Return to main, reset state
            m.state = UpdaterState::Main;
            m.checked.clear();
            m.total_update = 0;
            m.updating_remaining = 0;

            // Reset statuses
            for item in m.items.iter_mut() {
                if item.status == ToolStatus::Updated || item.status == ToolStatus::Failed {
                    item.message.clear();
                    if item.local_version == "MISSING" {
                        item.status = ToolStatus::Missing;
                    } else if item.remote_version != "..."
                        && item.remote_version != "Checking..."
                        && item.remote_version != "Unknown"
                        && item.remote_version != item.local_version
                    {
                        item.status = ToolStatus::Outdated;
                    } else {
                        item.status = ToolStatus::Installed;
                    }
                }
            }
            None
        }

        UpdaterState::Main => match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.should_quit = true;
                Some(Action::Quit)
            }
            KeyCode::Esc => {
                if !m.search_query.is_empty() {
                    m.search_query.clear();
                    m.filtered_indices = None;
                    None
                } else {
                    app.should_quit = true;
                    Some(Action::Quit)
                }
            }
            KeyCode::Char('/') => {
                m.state = UpdaterState::Search;
                m.search_query.clear();
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if m.cursor > 0 {
                    m.cursor -= 1;
                    while !m.is_item_visible(m.cursor) && m.cursor > 0 {
                        m.cursor -= 1;
                    }
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if m.cursor < m.items.len() - 1 {
                    m.cursor += 1;
                    while !m.is_item_visible(m.cursor) && m.cursor < m.items.len() - 1 {
                        m.cursor += 1;
                    }
                }
                None
            }
            KeyCode::Char('c') | KeyCode::Char('C') => {
                m.jump_to_category(Category::Code);
                None
            }
            KeyCode::Char('t') | KeyCode::Char('T') => {
                m.jump_to_category(Category::Term);
                None
            }
            KeyCode::Char('i') | KeyCode::Char('I') => {
                m.jump_to_category(Category::Ide);
                None
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                m.jump_to_category(Category::Prod);
                None
            }
            KeyCode::Char('f') | KeyCode::Char('F') => {
                m.jump_to_category(Category::Infra);
                None
            }
            KeyCode::Char('u') | KeyCode::Char('U') => {
                m.jump_to_category(Category::Utils);
                None
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                m.jump_to_category(Category::Runtime);
                None
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                m.jump_to_category(Category::Sys);
                None
            }
            KeyCode::Char(' ') => {
                if m.checked.contains(&m.cursor) {
                    m.checked.remove(&m.cursor);
                } else {
                    m.checked.insert(m.cursor);
                }
                None
            }
            KeyCode::Char('g') | KeyCode::Char('G') | KeyCode::Char('a') | KeyCode::Char('A') => {
                let current_cat = m.items[m.cursor].tool.category;
                let all_selected = m
                    .items
                    .iter()
                    .enumerate()
                    .filter(|(_, item)| item.tool.category == current_cat)
                    .all(|(i, _)| m.checked.contains(&i));

                for (i, item) in m.items.iter().enumerate() {
                    if item.tool.category == current_cat {
                        if all_selected {
                            m.checked.remove(&i);
                        } else {
                            m.checked.insert(i);
                        }
                    }
                }
                None
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if m.loading_count > 0 {
                    return None;
                }
                if m.checked.is_empty() {
                    m.checked.insert(m.cursor);
                }
                m.state = UpdaterState::Preview;
                None
            }
            KeyCode::Enter => {
                if m.loading_count > 0 {
                    return None;
                }
                if m.checked.is_empty() {
                    m.checked.insert(m.cursor);
                }
                if m.has_critical_selected() {
                    m.state = UpdaterState::Confirm;
                    None
                } else {
                    m.state = UpdaterState::Updating;
                    start_updates(m)
                }
            }
            _ => None,
        },
    }
}

/// Build the visual display order for port scanner: dev (sorted by project) then system (sorted by process)
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

    // Dev: project name first (with project before without), then port
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

    // System: process name then port
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

fn start_updates(m: &mut UpdaterModel) -> Option<Action> {
    m.build_update_queue();
    if m.update_queue.is_empty() {
        m.state = UpdaterState::Summary;
        return None;
    }

    if let Some(first) = m.update_queue.pop_front() {
        m.current_update = Some(first);
        m.current_log = UpdaterModel::get_update_log_text(&m.items[first].tool);
        return Some(Action::StartUpdate(first));
    }
    None
}
