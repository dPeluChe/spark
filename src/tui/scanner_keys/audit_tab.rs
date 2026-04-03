//! Key handlers for Security Audit tab.

use crossterm::event::{KeyCode, KeyEvent};
use crate::tui::model::*;
use crate::tui::update::Action;

pub fn handle(app: &mut App, key: KeyEvent) -> Option<Action> {
    let s = &mut app.scanner;

    match s.state {
        ScannerState::SecretAuditScanning => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { s.state = ScannerState::SecretAudit; None }
            _ => None,
        },

        ScannerState::SecretAudit => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { s.state = ScannerState::ScanConfig; None }
            KeyCode::Up | KeyCode::Char('k') => {
                if app.audit.cursor > 0 { app.audit.cursor -= 1; } None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if app.audit.cursor < app.audit.results.len().saturating_sub(1) { app.audit.cursor += 1; } None
            }
            KeyCode::Enter => {
                if !app.audit.results.is_empty() {
                    app.audit.detail_cursor = 0;
                    s.state = ScannerState::SecretAuditDetail;
                } None
            }
            KeyCode::Char('r') | KeyCode::Char('R') => {
                let path = std::env::current_dir().unwrap_or_else(|_| app.config.repos_root.clone());
                Some(Action::StartAudit(path))
            }
            _ => None,
        },

        ScannerState::SecretAuditDetail => match key.code {
            KeyCode::Esc | KeyCode::Char('q') => { s.state = ScannerState::SecretAudit; None }
            KeyCode::Up | KeyCode::Char('k') => {
                if app.audit.detail_cursor > 0 { app.audit.detail_cursor -= 1; } None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some(result) = app.audit.results.get(app.audit.cursor) {
                    if app.audit.detail_cursor < result.findings.len().saturating_sub(1) {
                        app.audit.detail_cursor += 1;
                    }
                } None
            }
            KeyCode::PageUp => {
                app.audit.detail_cursor = app.audit.detail_cursor.saturating_sub(super::PAGE_JUMP); None
            }
            KeyCode::PageDown => {
                if let Some(result) = app.audit.results.get(app.audit.cursor) {
                    app.audit.detail_cursor = (app.audit.detail_cursor + super::PAGE_JUMP)
                        .min(result.findings.len().saturating_sub(1));
                } None
            }
            _ => None,
        },

        _ => None,
    }
}
