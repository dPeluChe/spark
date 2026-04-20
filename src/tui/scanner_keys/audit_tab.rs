//! Key handlers for Security Audit tab.

use crate::tui::model::*;
use crate::tui::update::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(app: &mut App, key: KeyEvent) -> Option<Action> {
    let s = &mut app.scanner;

    match s.state {
        ScannerState::SecretAuditScanning => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::SecretAudit;
                None
            }
            _ => None,
        },

        ScannerState::SecretAudit => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::ScanConfig;
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if app.audit.cursor > 0 {
                    app.audit.cursor -= 1;
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                // +1 for the deps entry if there are dep vulns
                let max =
                    app.audit.results.len() + if app.audit.dep_vulns.is_empty() { 0 } else { 1 };
                if app.audit.cursor < max.saturating_sub(1) {
                    app.audit.cursor += 1;
                }
                None
            }
            KeyCode::Enter => {
                let dep_row = app.audit.results.len(); // deps entry is after all projects
                if !app.audit.dep_vulns.is_empty() && app.audit.cursor == dep_row {
                    app.audit.dep_cursor = 0;
                    s.state = ScannerState::SecretAuditDeps;
                } else if !app.audit.results.is_empty() && app.audit.cursor < dep_row {
                    app.audit.detail_cursor = 0;
                    s.state = ScannerState::SecretAuditDetail;
                }
                None
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                app.audit.path_input.clear();
                s.state = ScannerState::SecretAuditPathInput;
                None
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                let path = app.audit.scan_path.clone().unwrap_or_else(|| {
                    std::env::current_dir().unwrap_or_else(|_| app.config.repos_root.clone())
                });
                Some(Action::StartAudit(path))
            }
            _ => None,
        },

        ScannerState::SecretAuditDetail => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::SecretAudit;
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if app.audit.detail_cursor > 0 {
                    app.audit.detail_cursor -= 1;
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(result) = app.audit.results.get(app.audit.cursor) {
                    if app.audit.detail_cursor < result.findings.len().saturating_sub(1) {
                        app.audit.detail_cursor += 1;
                    }
                }
                None
            }
            KeyCode::PageUp => {
                app.audit.detail_cursor = app.audit.detail_cursor.saturating_sub(super::PAGE_JUMP);
                None
            }
            KeyCode::PageDown => {
                if let Some(result) = app.audit.results.get(app.audit.cursor) {
                    app.audit.detail_cursor = (app.audit.detail_cursor + super::PAGE_JUMP)
                        .min(result.findings.len().saturating_sub(1));
                }
                None
            }
            _ => None,
        },

        ScannerState::SecretAuditDeps => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::SecretAudit;
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if app.audit.dep_cursor > 0 {
                    app.audit.dep_cursor -= 1;
                }
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.audit.dep_cursor < app.audit.dep_vulns.len().saturating_sub(1) {
                    app.audit.dep_cursor += 1;
                }
                None
            }
            KeyCode::PageUp => {
                app.audit.dep_cursor = app.audit.dep_cursor.saturating_sub(super::PAGE_JUMP);
                None
            }
            KeyCode::PageDown => {
                app.audit.dep_cursor = (app.audit.dep_cursor + super::PAGE_JUMP)
                    .min(app.audit.dep_vulns.len().saturating_sub(1));
                None
            }
            _ => None,
        },

        ScannerState::SecretAuditPathInput => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                s.state = ScannerState::SecretAudit;
                None
            }
            KeyCode::Enter => {
                let input = app.audit.path_input.trim().to_string();
                if !input.is_empty() {
                    let path = crate::utils::fs::expand_tilde(&input);
                    if path.exists() && path.is_dir() {
                        app.audit.scan_path = Some(path.clone());
                        s.state = ScannerState::SecretAuditScanning;
                        return Some(Action::StartAudit(path));
                    }
                }
                s.state = ScannerState::SecretAudit;
                None
            }
            KeyCode::Backspace => {
                app.audit.path_input.pop();
                None
            }
            KeyCode::Char(c) => {
                app.audit.path_input.push(c);
                None
            }
            _ => None,
        },

        _ => None,
    }
}
