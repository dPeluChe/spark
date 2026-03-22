use crossterm::event::{KeyCode, KeyEvent};
use crate::tui::model::*;
use crate::tui::update::Action;

/// Handle key events for Scanner mode
pub fn handle_scanner_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    let s = &mut app.scanner;

    match s.state {
        ScannerState::ScanConfig => match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.should_quit = true;
                Some(Action::Quit)
            }
            KeyCode::Tab => {
                app.mode = AppMode::Updater;
                None
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                s.state = ScannerState::PortScan;
                Some(Action::ScanPorts)
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                s.state = ScannerState::RepoManager;
                Some(Action::ListManagedRepos)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if s.cursor > 0 {
                    s.cursor -= 1;
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if s.cursor < s.discovered_dirs.len().saturating_sub(1) {
                    s.cursor += 1;
                }
                None
            }
            KeyCode::Char(' ') => {
                if s.selected_scan_dirs.contains(&s.cursor) {
                    s.selected_scan_dirs.remove(&s.cursor);
                } else {
                    s.selected_scan_dirs.insert(s.cursor);
                }
                None
            }
            KeyCode::Enter => {
                let dirs: Vec<std::path::PathBuf> = s
                    .selected_scan_dirs
                    .iter()
                    .filter_map(|&i| s.discovered_dirs.get(i).cloned())
                    .collect();
                if !dirs.is_empty() {
                    s.state = ScannerState::Scanning;
                    return Some(Action::StartScan(dirs));
                }
                None
            }
            _ => None,
        },

        ScannerState::Scanning => None,

        ScannerState::ScanResults => match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                app.should_quit = true;
                Some(Action::Quit)
            }
            KeyCode::Tab => {
                app.mode = AppMode::Updater;
                None
            }
            KeyCode::Esc => {
                s.state = ScannerState::ScanConfig;
                None
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                s.state = ScannerState::PortScan;
                Some(Action::ScanPorts)
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                s.state = ScannerState::RepoManager;
                Some(Action::ListManagedRepos)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if s.cursor > 0 {
                    s.cursor -= 1;
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if s.cursor < s.repos.len().saturating_sub(1) {
                    s.cursor += 1;
                }
                None
            }
            KeyCode::Char(' ') => {
                if s.checked.contains(&s.cursor) {
                    s.checked.remove(&s.cursor);
                } else {
                    s.checked.insert(s.cursor);
                }
                None
            }
            KeyCode::Enter => {
                if !s.repos.is_empty() {
                    s.state = ScannerState::RepoDetail;
                }
                None
            }
            KeyCode::Char('d') => {
                if s.checked.is_empty() {
                    s.checked.insert(s.cursor);
                }
                let paths: Vec<std::path::PathBuf> = s
                    .checked
                    .iter()
                    .flat_map(|&i| {
                        s.repos
                            .get(i)
                            .map(|r| r.artifacts.iter().map(|a| a.path.clone()).collect::<Vec<_>>())
                            .unwrap_or_default()
                    })
                    .collect();
                if !paths.is_empty() {
                    s.state = ScannerState::CleanConfirm;
                }
                None
            }
            KeyCode::Char('D') => {
                if s.repos.get(s.cursor).is_some() {
                    s.state = ScannerState::CleanConfirm;
                }
                None
            }
            KeyCode::Char('s') => {
                s.sort_by = match s.sort_by {
                    SortField::Name => SortField::Health,
                    SortField::Health => SortField::LastCommit,
                    SortField::LastCommit => SortField::Size,
                    SortField::Size => SortField::ArtifactSize,
                    SortField::ArtifactSize => SortField::Name,
                };
                s.sort_repos();
                None
            }
            KeyCode::Char('r') => {
                s.sort_ascending = !s.sort_ascending;
                s.sort_repos();
                None
            }
            _ => None,
        },

        ScannerState::RepoDetail => match key.code {
            KeyCode::Esc => {
                s.state = ScannerState::ScanResults;
                None
            }
            KeyCode::Char('d') => {
                if let Some(repo) = s.repos.get(s.cursor) {
                    let paths: Vec<std::path::PathBuf> =
                        repo.artifacts.iter().map(|a| a.path.clone()).collect();
                    if !paths.is_empty() {
                        return Some(Action::CleanArtifacts(paths));
                    }
                }
                None
            }
            KeyCode::Char('D') => {
                if let Some(repo) = s.repos.get(s.cursor) {
                    return Some(Action::TrashRepo(repo.path.clone()));
                }
                None
            }
            _ => None,
        },

        ScannerState::CleanConfirm => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                s.state = ScannerState::Cleaning;
                let paths: Vec<std::path::PathBuf> = s
                    .checked
                    .iter()
                    .flat_map(|&i| {
                        s.repos
                            .get(i)
                            .map(|r| r.artifacts.iter().map(|a| a.path.clone()).collect::<Vec<_>>())
                            .unwrap_or_default()
                    })
                    .collect();
                Some(Action::CleanArtifacts(paths))
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                s.state = ScannerState::ScanResults;
                None
            }
            _ => None,
        },

        ScannerState::Cleaning => None,

        ScannerState::CleanSummary => match key.code {
            KeyCode::Enter | KeyCode::Esc => {
                s.state = ScannerState::ScanResults;
                s.checked.clear();
                s.clean_results.clear();
                None
            }
            _ => None,
        },

        // Port scanner states
        ScannerState::PortScan => {
            let p = &mut app.port_scanner;
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    app.should_quit = true;
                    Some(Action::Quit)
                }
                KeyCode::Esc => {
                    app.scanner.state = ScannerState::ScanConfig;
                    None
                }
                KeyCode::Tab => {
                    app.mode = AppMode::Updater;
                    None
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if p.cursor > 0 {
                        p.cursor -= 1;
                    }
                    None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if p.cursor < p.ports.len().saturating_sub(1) {
                        p.cursor += 1;
                    }
                    None
                }
                KeyCode::Char(' ') => {
                    if p.checked.contains(&p.cursor) {
                        p.checked.remove(&p.cursor);
                    } else {
                        p.checked.insert(p.cursor);
                    }
                    None
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    Some(Action::ScanPorts)
                }
                KeyCode::Char('x') => {
                    // Kill selected (or current)
                    if p.checked.is_empty() {
                        p.checked.insert(p.cursor);
                    }
                    app.scanner.state = ScannerState::PortKillConfirm;
                    None
                }
                KeyCode::Char('X') => {
                    // Kill all dev ports
                    for (i, port_info) in p.ports.iter().enumerate() {
                        if crate::scanner::port_scanner::is_dev_port(port_info.port) {
                            p.checked.insert(i);
                        }
                    }
                    if !p.checked.is_empty() {
                        app.scanner.state = ScannerState::PortKillConfirm;
                    }
                    None
                }
                _ => None,
            }
        }

        // Repo manager state
        ScannerState::RepoManager => {
            let rm = &mut app.repo_manager;
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    app.should_quit = true;
                    Some(Action::Quit)
                }
                KeyCode::Esc => {
                    app.scanner.state = ScannerState::ScanConfig;
                    None
                }
                KeyCode::Tab => {
                    app.mode = AppMode::Updater;
                    None
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if rm.cursor > 0 {
                        rm.cursor -= 1;
                    }
                    None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if rm.cursor < rm.repos.len().saturating_sub(1) {
                        rm.cursor += 1;
                    }
                    None
                }
                KeyCode::Char(' ') => {
                    if rm.checked.contains(&rm.cursor) {
                        rm.checked.remove(&rm.cursor);
                    } else {
                        rm.checked.insert(rm.cursor);
                    }
                    None
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    Some(Action::ListManagedRepos)
                }
                KeyCode::Char('u') => {
                    // Pull selected repos
                    if rm.checked.is_empty() {
                        rm.checked.insert(rm.cursor);
                    }
                    let indices: Vec<usize> = rm.checked.iter().copied().collect();
                    Some(Action::PullRepos(indices))
                }
                KeyCode::Char('U') => {
                    // Pull all repos that are behind
                    let behind_indices: Vec<usize> = rm
                        .repos
                        .iter()
                        .enumerate()
                        .filter(|(_, r)| {
                            matches!(
                                r.status,
                                crate::scanner::repo_manager::RepoStatus::Behind(_)
                                    | crate::scanner::repo_manager::RepoStatus::Diverged { .. }
                            )
                        })
                        .map(|(i, _)| i)
                        .collect();
                    if !behind_indices.is_empty() {
                        for &i in &behind_indices {
                            rm.checked.insert(i);
                        }
                        Some(Action::PullRepos(behind_indices))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }

        ScannerState::PortKillConfirm => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let pids: Vec<u32> = app
                    .port_scanner
                    .checked
                    .iter()
                    .filter_map(|&i| app.port_scanner.ports.get(i).map(|p| p.pid))
                    .collect();
                app.scanner.state = ScannerState::PortScan;
                if !pids.is_empty() {
                    Some(Action::KillProcesses(pids))
                } else {
                    None
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                app.scanner.state = ScannerState::PortScan;
                app.port_scanner.checked.clear();
                None
            }
            _ => None,
        },
    }
}
