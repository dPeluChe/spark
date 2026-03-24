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
            KeyCode::Char('p') | KeyCode::Char('P') => {
                s.state = ScannerState::PortScan;
                Some(Action::ScanPorts)
            }
            KeyCode::Char('g') | KeyCode::Char('G') => {
                s.state = ScannerState::RepoManager;
                Some(Action::ListManagedRepos)
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if s.cursor > 0 { s.cursor -= 1; }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if s.cursor < s.discovered_dirs.len().saturating_sub(1) { s.cursor += 1; }
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
            KeyCode::Char('r') | KeyCode::Char('R') => {
                // Refresh: re-discover directories with updated counts
                s.discovered_dirs.clear();
                s.selected_scan_dirs.clear();
                s.cursor = 0;
                Some(Action::DiscoverDirs)
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                s.path_input.clear();
                s.state = ScannerState::ScanAddPath;
                None
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                if s.cursor < s.discovered_dirs.len() {
                    s.discovered_dirs.remove(s.cursor);
                    s.selected_scan_dirs.clear();
                    if s.cursor > 0 && s.cursor >= s.discovered_dirs.len() {
                        s.cursor -= 1;
                    }
                }
                None
            }
            KeyCode::Enter => {
                let mut dirs: Vec<std::path::PathBuf> = s
                    .selected_scan_dirs
                    .iter()
                    .filter_map(|&i| s.discovered_dirs.get(i).map(|d| d.path.clone()))
                    .collect();
                if dirs.is_empty() {
                    if let Some(d) = s.discovered_dirs.get(s.cursor) {
                        dirs.push(d.path.clone());
                    }
                }
                if !dirs.is_empty() {
                    s.state = ScannerState::Scanning;
                    return Some(Action::StartScan(dirs));
                }
                None
            }
            _ => None,
        },

        ScannerState::ScanAddPath => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::ScanConfig;
                None
            }
            KeyCode::Enter => {
                let input = s.path_input.trim().to_string();
                if !input.is_empty() {
                    let path = if input.starts_with('~') {
                        let home = dirs::home_dir().unwrap_or_default();
                        home.join(input.strip_prefix("~/").unwrap_or(&input))
                    } else {
                        std::path::PathBuf::from(&input)
                    };
                    if path.exists() && path.is_dir() {
                        let idx = s.discovered_dirs.len();
                        let repo_count = crate::scanner::repo_scanner::count_repos_in(&path);
                        s.discovered_dirs.push(crate::scanner::repo_scanner::DiscoveredDir {
                            path, repo_count,
                        });
                        s.selected_scan_dirs.insert(idx);
                    }
                }
                s.state = ScannerState::ScanConfig;
                None
            }
            KeyCode::Backspace => { s.path_input.pop(); None }
            KeyCode::Char(c) => { s.path_input.push(c); None }
            _ => None,
        },

        ScannerState::ContainerLoading => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::ScanResults;
                None
            }
            _ => None,
        },

        ScannerState::Scanning => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::ScanConfig;
                None
            }
            _ => None,
        },

        // q = back to ScanConfig, not quit
        ScannerState::ScanResults => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::ScanConfig;
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if s.cursor > 0 { s.cursor -= 1; }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if s.cursor < s.repos.len().saturating_sub(1) { s.cursor += 1; }
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
                if let Some(repo) = s.repos.get(s.cursor) {
                    if repo.is_container {
                        s.container_children.clear();
                        s.container_cursor = 0;
                        s.state = ScannerState::ContainerLoading;
                        return Some(Action::LoadContainerChildren(repo.path.clone()));
                    }
                    s.state = ScannerState::RepoDetail;
                }
                None
            }
            KeyCode::Char('c') => {
                if s.checked.is_empty() { s.checked.insert(s.cursor); }
                let paths: Vec<std::path::PathBuf> = s.checked.iter()
                    .flat_map(|&i| {
                        s.repos.get(i)
                            .map(|r| r.artifacts.iter().map(|a| a.path.clone()).collect::<Vec<_>>())
                            .unwrap_or_default()
                    })
                    .collect();
                if !paths.is_empty() { s.state = ScannerState::CleanConfirm; }
                None
            }
            KeyCode::Char('x') => {
                if s.repos.get(s.cursor).is_some() {
                    s.state = ScannerState::DeleteRepoConfirm;
                }
                None
            }
            KeyCode::Char('?') | KeyCode::Char('h') => {
                s.state = ScannerState::HealthHelp;
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

        // q = back to results
        ScannerState::RepoDetail => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::ScanResults;
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if s.container_cursor > 0 { s.container_cursor -= 1; }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !s.container_children.is_empty()
                    && s.container_cursor < s.container_children.len().saturating_sub(1)
                {
                    s.container_cursor += 1;
                }
                None
            }
            KeyCode::Char('s') => {
                s.container_sort = (s.container_sort + 1) % 4;
                let sort = s.container_sort;
                s.container_children.sort_by(|a, b| match sort {
                    0 => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                    1 => a.health_score.cmp(&b.health_score),
                    2 => b.last_commit_date.cmp(&a.last_commit_date),
                    _ => b.artifact_size.cmp(&a.artifact_size),
                });
                s.container_cursor = 0;
                let label = match sort { 0 => "name", 1 => "health", 2 => "recent", _ => "size" };
                app.show_toast(format!("Sorted by {}", label), false);
                None
            }
            KeyCode::Char('a') => {
                // Add this repo's path as a scan directory (stay in detail)
                let info = s.repos.get(s.cursor).map(|r| (r.path.clone(), r.name.clone(), r.child_repo_count));
                if let Some((path, name, child_count)) = info {
                    let already = s.discovered_dirs.iter().any(|d| d.path == path);
                    if !already {
                        let idx = s.discovered_dirs.len();
                        s.discovered_dirs.push(crate::scanner::repo_scanner::DiscoveredDir {
                            path, repo_count: child_count,
                        });
                        s.selected_scan_dirs.insert(idx);
                        app.show_toast(format!("Added {} to scan paths ({} repos)", name, child_count), false);
                    } else {
                        app.show_toast(format!("{} already in scan paths", name), false);
                    }
                }
                None
            }
            KeyCode::Char('c') => {
                if let Some(repo) = s.repos.get(s.cursor) {
                    let paths: Vec<std::path::PathBuf> =
                        repo.artifacts.iter().map(|a| a.path.clone()).collect();
                    if !paths.is_empty() {
                        return Some(Action::CleanArtifacts(paths));
                    }
                }
                None
            }
            KeyCode::Char('x') => {
                if s.repos.get(s.cursor).is_some() {
                    s.state = ScannerState::DeleteRepoConfirm;
                }
                None
            }
            _ => None,
        },

        ScannerState::HealthHelp => match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter | KeyCode::Char('?') => {
                s.state = ScannerState::ScanResults;
                None
            }
            _ => None,
        },

        ScannerState::CleanConfirm => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                s.state = ScannerState::Cleaning;
                let paths: Vec<std::path::PathBuf> = s.checked.iter()
                    .flat_map(|&i| {
                        s.repos.get(i)
                            .map(|r| r.artifacts.iter().map(|a| a.path.clone()).collect::<Vec<_>>())
                            .unwrap_or_default()
                    })
                    .collect();
                Some(Action::CleanArtifacts(paths))
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::ScanResults;
                None
            }
            _ => None,
        },

        ScannerState::DeleteRepoConfirm => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(repo) = s.repos.get(s.cursor) {
                    let path = repo.path.clone();
                    let name = repo.name.clone();
                    // Remove from list immediately
                    s.repos.remove(s.cursor);
                    if s.cursor > 0 && s.cursor >= s.repos.len() {
                        s.cursor -= 1;
                    }
                    s.state = ScannerState::ScanResults;
                    app.show_toast(format!("Deleted {}", name), false);
                    return Some(Action::TrashRepo(path));
                }
                s.state = ScannerState::ScanResults;
                None
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::ScanResults;
                None
            }
            _ => None,
        },

        ScannerState::Cleaning => None,

        // Port scanner: q = back to ScanConfig
        ScannerState::PortScan => {
            let p = &mut app.port_scanner;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    app.scanner.state = ScannerState::ScanConfig;
                    None
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if p.cursor > 0 { p.cursor -= 1; }
                    None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if p.cursor < p.display_order.len().saturating_sub(1) { p.cursor += 1; }
                    None
                }
                KeyCode::Char(' ') => {
                    if let Some(idx) = p.cursor_port_index() {
                        if p.checked.contains(&idx) { p.checked.remove(&idx); }
                        else { p.checked.insert(idx); }
                    }
                    None
                }
                KeyCode::Enter => {
                    if p.cursor_port_index().is_some() {
                        app.scanner.state = ScannerState::PortAction;
                    }
                    None
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    Some(Action::ScanPorts)
                }
                KeyCode::Char('x') => {
                    if let Some(idx) = p.cursor_port_index() {
                        if p.checked.is_empty() { p.checked.insert(idx); }
                        app.scanner.state = ScannerState::PortKillConfirm;
                    }
                    None
                }
                KeyCode::Char('X') => {
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

        // Port action modal: q = close
        ScannerState::PortAction => match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                app.scanner.state = ScannerState::PortScan;
                None
            }
            KeyCode::Char('k') => {
                if let Some(idx) = app.port_scanner.cursor_port_index() {
                    if let Some(port_info) = app.port_scanner.ports.get(idx) {
                        let pid = port_info.pid;
                        app.scanner.state = ScannerState::PortScan;
                        return Some(Action::KillProcesses(vec![pid]));
                    }
                }
                app.scanner.state = ScannerState::PortScan;
                None
            }
            KeyCode::Char('o') => {
                if let Some(idx) = app.port_scanner.cursor_port_index() {
                    if let Some(port_info) = app.port_scanner.ports.get(idx) {
                        if let Some(ref cwd) = port_info.cwd {
                            let path = cwd.clone();
                            app.scanner.state = ScannerState::PortScan;
                            return Some(Action::OpenDir(path));
                        }
                    }
                }
                app.scanner.state = ScannerState::PortScan;
                None
            }
            _ => None,
        },

        ScannerState::PortKillConfirm => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let pids: Vec<u32> = app.port_scanner.checked.iter()
                    .filter_map(|&i| app.port_scanner.ports.get(i).map(|p| p.pid))
                    .collect();
                app.scanner.state = ScannerState::PortScan;
                if !pids.is_empty() { Some(Action::KillProcesses(pids)) } else { None }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
                app.scanner.state = ScannerState::PortScan;
                app.port_scanner.checked.clear();
                None
            }
            _ => None,
        },

        // Repo manager: q = back to ScanConfig
        ScannerState::RepoManager => {
            let rm = &mut app.repo_manager;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    app.scanner.state = ScannerState::ScanConfig;
                    None
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if rm.cursor > 0 { rm.cursor -= 1; }
                    None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if rm.cursor < rm.repos.len().saturating_sub(1) { rm.cursor += 1; }
                    None
                }
                KeyCode::Char(' ') => {
                    if rm.checked.contains(&rm.cursor) { rm.checked.remove(&rm.cursor); }
                    else { rm.checked.insert(rm.cursor); }
                    None
                }
                KeyCode::Enter => {
                    if !rm.repos.is_empty() {
                        app.scanner.state = ScannerState::RepoAction;
                    }
                    None
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    Some(Action::ListManagedRepos)
                }
                KeyCode::Char('c') | KeyCode::Char('C') => {
                    app.repo_manager.clone_input.clear();
                    app.repo_manager.clone_error = None;
                    app.scanner.state = ScannerState::RepoCloneInput;
                    None
                }
                KeyCode::Char('u') => {
                    if rm.checked.is_empty() { rm.checked.insert(rm.cursor); }
                    let indices: Vec<usize> = rm.checked.iter().copied().collect();
                    Some(Action::PullRepos(indices))
                }
                KeyCode::Char('U') => {
                    let behind: Vec<usize> = rm.repos.iter().enumerate()
                        .filter(|(_, r)| matches!(r.status,
                            crate::scanner::repo_manager::RepoStatus::Behind(_)
                            | crate::scanner::repo_manager::RepoStatus::Diverged { .. }
                        ))
                        .map(|(i, _)| i).collect();
                    if !behind.is_empty() {
                        for &i in &behind { rm.checked.insert(i); }
                        Some(Action::PullRepos(behind))
                    } else { None }
                }
                _ => None,
            }
        }

        // Repo action modal: q = close
        ScannerState::RepoAction => match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => {
                app.scanner.state = ScannerState::RepoManager;
                None
            }
            KeyCode::Char('u') => {
                let idx = app.repo_manager.cursor;
                app.scanner.state = ScannerState::RepoManager;
                Some(Action::PullRepos(vec![idx]))
            }
            KeyCode::Char('d') => {
                if let Some(repo) = app.repo_manager.repos.get(app.repo_manager.cursor) {
                    let path = repo.path.clone();
                    app.scanner.state = ScannerState::RepoManager;
                    return Some(Action::TrashRepo(path));
                }
                app.scanner.state = ScannerState::RepoManager;
                None
            }
            KeyCode::Char('o') => {
                if let Some(repo) = app.repo_manager.repos.get(app.repo_manager.cursor) {
                    let path = repo.path.clone();
                    app.scanner.state = ScannerState::RepoManager;
                    return Some(Action::OpenDir(path));
                }
                app.scanner.state = ScannerState::RepoManager;
                None
            }
            _ => None,
        },

        ScannerState::RepoCloneSummary => match key.code {
            KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => {
                app.scanner.state = ScannerState::RepoManager;
                Some(Action::ListManagedRepos)
            }
            _ => None,
        },

        ScannerState::RepoCloneInput => {
            let rm = &mut app.repo_manager;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    rm.clone_input.clear();
                    rm.clone_error = None;
                    app.scanner.state = ScannerState::RepoManager;
                    None
                }
                KeyCode::Enter => {
                    let url = rm.clone_input.trim().to_string();
                    if url.is_empty() {
                        rm.clone_error = Some("URL cannot be empty".into());
                        None
                    } else if !url.contains("github.com")
                        && !url.contains("gitlab.com")
                        && !url.contains("bitbucket.org")
                        && !url.starts_with("git@")
                        && !url.starts_with("https://")
                    {
                        rm.clone_error = Some("Invalid git URL".into());
                        None
                    } else {
                        rm.cloning = true;
                        rm.clone_error = None;
                        Some(Action::CloneRepo(url))
                    }
                }
                KeyCode::Backspace => { rm.clone_input.pop(); rm.clone_error = None; None }
                KeyCode::Char(c) => { rm.clone_input.push(c); rm.clone_error = None; None }
                _ => None,
            }
        }
    }
}
