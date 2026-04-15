//! Key handlers for Scanner tab: scan config, results, repo detail, container, clean, delete.

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crate::tui::model::*;
use crate::tui::update::Action;

pub fn handle(app: &mut App, key: KeyEvent) -> Option<Action> {
    let s = &mut app.scanner;

    match s.state {
        ScannerState::ScanConfig => match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => {
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

        ScannerState::ScanResults => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { s.state = ScannerState::ScanConfig; None }
            KeyCode::Up | KeyCode::Char('k') => { if s.cursor > 0 { s.cursor -= 1; } None }
            KeyCode::Down | KeyCode::Char('j') => {
                if s.cursor < s.repos.len().saturating_sub(1) { s.cursor += 1; } None
            }
            KeyCode::Home => { s.cursor = 0; None }
            KeyCode::End => { s.cursor = s.repos.len().saturating_sub(1); None }
            KeyCode::PageUp => { s.cursor = s.cursor.saturating_sub(super::PAGE_JUMP); None }
            KeyCode::PageDown => {
                s.cursor = (s.cursor + super::PAGE_JUMP).min(s.repos.len().saturating_sub(1)); None
            }
            KeyCode::Char(' ') => {
                if s.checked.contains(&s.cursor) { s.checked.remove(&s.cursor); }
                else { s.checked.insert(s.cursor); }
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
                    }).collect();
                if !paths.is_empty() { s.state = ScannerState::CleanConfirm; }
                None
            }
            KeyCode::Char('x') => {
                if s.repos.get(s.cursor).is_some() { s.state = ScannerState::DeleteRepoConfirm; }
                None
            }
            KeyCode::Char('?') | KeyCode::Char('h') => { s.state = ScannerState::HealthHelp; None }
            KeyCode::Char('s') => {
                s.sort_by = match s.sort_by {
                    SortField::Name => SortField::Health,
                    SortField::Health => SortField::LastCommit,
                    SortField::LastCommit => SortField::Size,
                    SortField::Size => SortField::ArtifactSize,
                    SortField::ArtifactSize => SortField::Name,
                };
                s.sort_repos(); None
            }
            KeyCode::Char('r') => { s.sort_ascending = !s.sort_ascending; s.sort_repos(); None }
            _ => None,
        },

        ScannerState::RepoDetail => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { s.state = ScannerState::ScanResults; None }
            KeyCode::Enter => {
                if s.repos.get(s.cursor).map(|r| r.is_container).unwrap_or(false)
                    && !s.container_children.is_empty()
                { s.state = ScannerState::ContainerChildDetail; }
                None
            }
            KeyCode::Up | KeyCode::Char('k') => { if s.container_cursor > 0 { s.container_cursor -= 1; } None }
            KeyCode::Down | KeyCode::Char('j') => {
                if !s.container_children.is_empty() && s.container_cursor < s.container_children.len().saturating_sub(1) {
                    s.container_cursor += 1;
                } None
            }
            KeyCode::Home => { s.container_cursor = 0; None }
            KeyCode::End => { s.container_cursor = s.container_children.len().saturating_sub(1); None }
            KeyCode::PageUp => { s.container_cursor = s.container_cursor.saturating_sub(super::PAGE_JUMP); None }
            KeyCode::PageDown => {
                s.container_cursor = (s.container_cursor + super::PAGE_JUMP).min(s.container_children.len().saturating_sub(1)); None
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
                let info = s.repos.get(s.cursor).map(|r| (r.path.clone(), r.name.clone(), r.child_repo_count));
                if let Some((path, name, child_count)) = info {
                    let already = s.discovered_dirs.iter().any(|d| d.path == path);
                    if !already {
                        let idx = s.discovered_dirs.len();
                        s.discovered_dirs.push(crate::scanner::repo_scanner::DiscoveredDir { path, repo_count: child_count });
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
                    let paths: Vec<std::path::PathBuf> = repo.artifacts.iter().map(|a| a.path.clone()).collect();
                    if !paths.is_empty() { return Some(Action::CleanArtifacts(paths)); }
                }
                None
            }
            KeyCode::Char('x') => {
                if s.repos.get(s.cursor).is_some() { s.state = ScannerState::DeleteRepoConfirm; }
                None
            }
            _ => None,
        },

        ScannerState::ContainerChildDetail => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { s.state = ScannerState::RepoDetail; None }
            KeyCode::Up | KeyCode::Char('k') => { if s.container_cursor > 0 { s.container_cursor -= 1; } None }
            KeyCode::Down | KeyCode::Char('j') => {
                if !s.container_children.is_empty() && s.container_cursor < s.container_children.len().saturating_sub(1) {
                    s.container_cursor += 1;
                } None
            }
            KeyCode::Char('c') => {
                if let Some(child) = s.container_children.get(s.container_cursor) {
                    let paths: Vec<std::path::PathBuf> = child.artifacts.iter().map(|a| a.path.clone()).collect();
                    if !paths.is_empty() { s.state = ScannerState::Cleaning; return Some(Action::CleanArtifacts(paths)); }
                } None
            }
            KeyCode::Char('x') => {
                if s.container_children.get(s.container_cursor).is_some() { s.state = ScannerState::ContainerChildDelete; }
                None
            }
            _ => None,
        },

        ScannerState::ContainerChildDelete => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(child) = s.container_children.get(s.container_cursor) {
                    let path = child.path.clone();
                    let name = child.name.clone();
                    s.container_children.remove(s.container_cursor);
                    if s.container_cursor > 0 && s.container_cursor >= s.container_children.len() { s.container_cursor -= 1; }
                    s.state = ScannerState::RepoDetail;
                    app.show_toast(format!("Deleted {}", name), false);
                    return Some(Action::TrashRepo(path));
                }
                s.state = ScannerState::RepoDetail; None
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::ContainerChildDetail; None
            }
            _ => None,
        },

        ScannerState::HealthHelp => match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter | KeyCode::Char('?') => {
                s.state = ScannerState::ScanResults; None
            }
            _ => None,
        },

        ScannerState::CleanConfirm => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                s.state = ScannerState::Cleaning;
                let paths: Vec<std::path::PathBuf> = s.checked.iter()
                    .flat_map(|&i| s.repos.get(i)
                        .map(|r| r.artifacts.iter().map(|a| a.path.clone()).collect::<Vec<_>>())
                        .unwrap_or_default()
                    ).collect();
                Some(Action::CleanArtifacts(paths))
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::ScanResults; None
            }
            _ => None,
        },

        ScannerState::DeleteRepoConfirm => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if let Some(repo) = s.repos.get(s.cursor) {
                    let path = repo.path.clone();
                    let name = repo.name.clone();
                    s.repos.remove(s.cursor);
                    if s.cursor > 0 && s.cursor >= s.repos.len() { s.cursor -= 1; }
                    s.state = ScannerState::ScanResults;
                    app.show_toast(format!("Deleted {}", name), false);
                    return Some(Action::TrashRepo(path));
                }
                s.state = ScannerState::ScanResults; None
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::ScanResults; None
            }
            _ => None,
        },

        ScannerState::Cleaning => None,

        _ => None,
    }
}
