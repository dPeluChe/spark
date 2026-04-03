//! Key handlers for System Cleanup tab.

use crossterm::event::{KeyCode, KeyEvent};
use crate::tui::model::*;
use crate::tui::update::Action;

pub fn handle(app: &mut App, key: KeyEvent) -> Option<Action> {
    match app.scanner.state {
        ScannerState::SystemClean => {
            let sc = &mut app.system_cleaner;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => { app.scanner.state = ScannerState::ScanConfig; None }
                KeyCode::Up | KeyCode::Char('k') => { if sc.cursor > 0 { sc.cursor -= 1; } None }
                KeyCode::Down | KeyCode::Char('j') => {
                    if sc.cursor < sc.items.len().saturating_sub(1) { sc.cursor += 1; } None
                }
                KeyCode::Char(' ') => {
                    if sc.checked.contains(&sc.cursor) { sc.checked.remove(&sc.cursor); }
                    else { sc.checked.insert(sc.cursor); }
                    None
                }
                KeyCode::Enter | KeyCode::Char('x') => {
                    if sc.items.get(sc.cursor).is_some() {
                        app.scanner.state = ScannerState::SystemCleanConfirm;
                    }
                    None
                }
                KeyCode::Char('r') | KeyCode::Char('R') => Some(Action::ScanSystem),
                _ => None,
            }
        }

        ScannerState::SystemCleanConfirm => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let idx = app.system_cleaner.cursor;
                app.scanner.state = ScannerState::SystemClean;
                Some(Action::CleanSystemItem(idx))
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
                app.scanner.state = ScannerState::SystemClean; None
            }
            _ => None,
        },

        _ => None,
    }
}
