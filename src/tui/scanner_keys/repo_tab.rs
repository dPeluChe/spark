//! Key handlers for Repo Manager tab.

use crossterm::event::{KeyCode, KeyEvent};
use crate::tui::model::*;
use crate::tui::update::Action;

pub fn handle(app: &mut App, key: KeyEvent) -> Option<Action> {
    match app.scanner.state {
        ScannerState::RepoManager => {
            let rm = &mut app.repo_manager;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => { app.scanner.state = ScannerState::ScanConfig; None }
                KeyCode::Up | KeyCode::Char('k') => { if rm.cursor > 0 { rm.cursor -= 1; } None }
                KeyCode::Down | KeyCode::Char('j') => {
                    if rm.cursor < rm.repos.len().saturating_sub(1) { rm.cursor += 1; } None
                }
                KeyCode::Char(' ') => {
                    if rm.checked.contains(&rm.cursor) { rm.checked.remove(&rm.cursor); }
                    else { rm.checked.insert(rm.cursor); }
                    None
                }
                KeyCode::Enter => {
                    if !rm.repos.is_empty() { app.scanner.state = ScannerState::RepoAction; }
                    None
                }
                KeyCode::Char('r') | KeyCode::Char('R') => {
                    crate::scanner::repo_manager::clear_status_cache();
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
                        )).map(|(i, _)| i).collect();
                    if !behind.is_empty() {
                        for &i in &behind { rm.checked.insert(i); }
                        Some(Action::PullRepos(behind))
                    } else { None }
                }
                _ => None,
            }
        }

        ScannerState::RepoAction => match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => { app.scanner.state = ScannerState::RepoManager; None }
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
                app.scanner.state = ScannerState::RepoManager; None
            }
            KeyCode::Char('o') => {
                if let Some(repo) = app.repo_manager.repos.get(app.repo_manager.cursor) {
                    let path = repo.path.clone();
                    app.scanner.state = ScannerState::RepoManager;
                    return Some(Action::OpenDir(path));
                }
                app.scanner.state = ScannerState::RepoManager; None
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
                    rm.clone_input.clear(); rm.clone_error = None;
                    app.scanner.state = ScannerState::RepoManager; None
                }
                KeyCode::Enter => {
                    let url = rm.clone_input.trim().to_string();
                    if url.is_empty() {
                        rm.clone_error = Some("URL cannot be empty".into()); None
                    } else if !url.contains("github.com") && !url.contains("gitlab.com")
                        && !url.contains("bitbucket.org") && !url.starts_with("git@")
                        && !url.starts_with("https://")
                    {
                        rm.clone_error = Some("Invalid git URL".into()); None
                    } else {
                        rm.cloning = true; rm.clone_error = None;
                        Some(Action::CloneRepo(url))
                    }
                }
                KeyCode::Backspace => { rm.clone_input.pop(); rm.clone_error = None; None }
                KeyCode::Char(c) => { rm.clone_input.push(c); rm.clone_error = None; None }
                _ => None,
            }
        }

        _ => None,
    }
}
