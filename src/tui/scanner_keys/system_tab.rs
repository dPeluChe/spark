//! Key handlers for System Cleanup tab.

use crate::tui::model::*;
use crate::tui::update::Action;
use crossterm::event::{KeyCode, KeyEvent};

pub fn handle(app: &mut App, key: KeyEvent) -> Option<Action> {
    match app.scanner.state {
        ScannerState::SystemClean => {
            let sc = &mut app.system_cleaner;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => {
                    app.scanner.state = ScannerState::ScanConfig;
                    None
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    sc.move_up();
                    None
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    sc.move_down();
                    None
                }
                KeyCode::Char(' ') => {
                    if sc.checked.contains(&sc.cursor) {
                        sc.checked.remove(&sc.cursor);
                    } else {
                        sc.checked.insert(sc.cursor);
                    }
                    None
                }
                KeyCode::Enter => {
                    if sc.items.get(sc.cursor).is_some() {
                        app.scanner.state = ScannerState::SystemCleanConfirm;
                    }
                    None
                }
                KeyCode::Char('x') | KeyCode::Char('X') => {
                    if !sc.checked.is_empty() {
                        app.scanner.state = ScannerState::SystemCleanConfirmBulk;
                    } else if sc.items.get(sc.cursor).is_some() {
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
                app.scanner.state = ScannerState::SystemClean;
                None
            }
            _ => None,
        },

        ScannerState::SystemCleanConfirmBulk => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let indices: Vec<usize> = app.system_cleaner.checked.iter().copied().collect();
                app.system_cleaner.checked.clear();
                app.scanner.state = ScannerState::SystemClean;
                Some(Action::CleanSystemItems(indices))
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
                app.scanner.state = ScannerState::SystemClean;
                None
            }
            _ => None,
        },

        _ => None,
    }
}
