//! Key handlers for Port Scanner tab.

use crossterm::event::{KeyCode, KeyEvent};
use crate::tui::model::*;
use crate::tui::update::Action;

pub fn handle(app: &mut App, key: KeyEvent) -> Option<Action> {
    match app.scanner.state {
        ScannerState::PortScan => {
            let p = &mut app.port_scanner;
            match key.code {
                KeyCode::Esc | KeyCode::Char('q') => { app.scanner.state = ScannerState::ScanConfig; None }
                KeyCode::Up | KeyCode::Char('k') => { if p.cursor > 0 { p.cursor -= 1; } None }
                KeyCode::Down | KeyCode::Char('j') => {
                    if p.cursor < p.display_order.len().saturating_sub(1) { p.cursor += 1; } None
                }
                KeyCode::Char(' ') => {
                    if let Some(idx) = p.cursor_port_index() {
                        if p.checked.contains(&idx) { p.checked.remove(&idx); }
                        else { p.checked.insert(idx); }
                    } None
                }
                KeyCode::Enter => {
                    if p.cursor_port_index().is_some() { app.scanner.state = ScannerState::PortAction; }
                    None
                }
                KeyCode::Char('r') | KeyCode::Char('R') => Some(Action::ScanPorts),
                KeyCode::Char('x') => {
                    if let Some(idx) = p.cursor_port_index() {
                        if p.checked.is_empty() { p.checked.insert(idx); }
                        app.scanner.state = ScannerState::PortKillConfirm;
                    } None
                }
                KeyCode::Char('X') => {
                    for (i, port_info) in p.ports.iter().enumerate() {
                        if crate::scanner::port_scanner::is_dev_port(port_info.port) { p.checked.insert(i); }
                    }
                    if !p.checked.is_empty() { app.scanner.state = ScannerState::PortKillConfirm; }
                    None
                }
                _ => None,
            }
        }

        ScannerState::PortAction => match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => { app.scanner.state = ScannerState::PortScan; None }
            KeyCode::Char('k') => {
                if let Some(idx) = app.port_scanner.cursor_port_index() {
                    if let Some(port_info) = app.port_scanner.ports.get(idx) {
                        let pid = port_info.pid;
                        app.scanner.state = ScannerState::PortScan;
                        return Some(Action::KillProcesses(vec![pid]));
                    }
                }
                app.scanner.state = ScannerState::PortScan; None
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
                app.scanner.state = ScannerState::PortScan; None
            }
            _ => None,
        },

        ScannerState::PortKillConfirm => match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let pids: Vec<u32> = app.port_scanner.checked.iter()
                    .filter_map(|&i| app.port_scanner.ports.get(i).map(|p| p.pid)).collect();
                app.scanner.state = ScannerState::PortScan;
                if !pids.is_empty() { Some(Action::KillProcesses(pids)) } else { None }
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
                app.scanner.state = ScannerState::PortScan;
                app.port_scanner.checked.clear(); None
            }
            _ => None,
        },

        _ => None,
    }
}
