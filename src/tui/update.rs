use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::core::types::*;
use crate::tui::model::*;
use crate::tui::scanner_keys;

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
}

/// Handle a key event and return optional action
pub fn handle_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    // Global: Ctrl+C always quits
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return Some(Action::Quit);
    }

    match app.mode {
        AppMode::Updater => handle_updater_key(app, key),
        AppMode::Scanner => scanner_keys::handle_scanner_key(app, key),
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
            app.scanner.state = ScannerState::CleanSummary;
        }
        AppMessage::PortScanResult { ports } => {
            app.port_scanner.ports = ports;
            app.port_scanner.cursor = 0;
            app.port_scanner.checked.clear();
        }
        AppMessage::KillResult { pid, success, error } => {
            if success {
                // Remove killed port from the list
                app.port_scanner.ports.retain(|p| p.pid != pid);
                if app.port_scanner.cursor >= app.port_scanner.ports.len()
                    && app.port_scanner.cursor > 0
                {
                    app.port_scanner.cursor -= 1;
                }
            }
            if let Some(err) = error {
                // Could show in a status bar, for now just log
                let _ = err;
            }
            // If all kills done and we were in confirm, go back to port scan
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
                if success {
                    app.repo_manager.repos[index].status =
                        crate::scanner::repo_manager::RepoStatus::UpToDate;
                } else {
                    app.repo_manager.repos[index].status =
                        crate::scanner::repo_manager::RepoStatus::Error(message);
                }
            }
            app.repo_manager.checked.remove(&index);
        }
        AppMessage::DiscoveredDirs { dirs } => {
            app.scanner.discovered_dirs = dirs;
            // Auto-select all discovered dirs
            for i in 0..app.scanner.discovered_dirs.len() {
                app.scanner.selected_scan_dirs.insert(i);
            }
        }
    }
    None
}

fn handle_updater_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    let m = &mut app.updater;

    match m.state {
        UpdaterState::Splash => {
            m.state = UpdaterState::Main;
            None
        }

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
            KeyCode::Tab => {
                app.mode = AppMode::Scanner;
                if app.scanner.discovered_dirs.is_empty() {
                    return Some(Action::DiscoverDirs);
                }
                None
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
