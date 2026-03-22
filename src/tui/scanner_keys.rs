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

        ScannerState::Scanning => None, // Block input during scan

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
                // Clean artifacts of selected repos
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
                // Trash selected repos
                if s.repos.get(s.cursor).is_some() {
                    s.state = ScannerState::CleanConfirm;
                }
                None
            }
            KeyCode::Char('s') => {
                // Cycle sort
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
    }
}
