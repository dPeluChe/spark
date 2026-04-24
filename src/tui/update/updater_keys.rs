//! Key bindings for the Updater tab.
//!
//! Non-updater tabs live in `tui::scanner_keys`. This module handles the
//! Updater states: Main (browse + select), Search (filter), Preview (dry run),
//! Confirm (critical runtime gate), Updating (blocked), Summary.

use super::Action;
use crate::core::types::*;
use crate::tui::model::*;
use crossterm::event::{KeyCode, KeyEvent};

pub(super) fn handle_updater_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    let m = &mut app.updater;

    match m.state {
        UpdaterState::Search => {
            handle_search(m, key);
            None
        }
        UpdaterState::Preview => handle_preview(m, key),
        UpdaterState::Confirm => handle_confirm(m, key),
        UpdaterState::Updating => None, // Block input (Ctrl+C handled globally)
        UpdaterState::Summary => {
            reset_after_summary(m);
            None
        }
        UpdaterState::Main => handle_main(app, key),
    }
}

fn handle_search(m: &mut UpdaterModel, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            m.state = UpdaterState::Main;
            m.search_query.clear();
            m.filtered_indices = None;
        }
        KeyCode::Enter => {
            m.state = UpdaterState::Main;
        }
        KeyCode::Backspace => {
            m.search_query.pop();
            m.update_filter();
        }
        KeyCode::Char(c) => {
            m.search_query.push(c);
            m.update_filter();
        }
        _ => {}
    }
}

fn handle_preview(m: &mut UpdaterModel, key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Enter => {
            if m.has_critical_selected() {
                m.state = UpdaterState::Confirm;
                None
            } else {
                m.state = UpdaterState::Updating;
                start_updates(m)
            }
        }
        KeyCode::Esc | KeyCode::Char('q') => {
            m.state = UpdaterState::Main;
            None
        }
        _ => None,
    }
}

fn handle_confirm(m: &mut UpdaterModel, key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            m.state = UpdaterState::Updating;
            start_updates(m)
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc | KeyCode::Char('q') => {
            m.state = UpdaterState::Main;
            None
        }
        _ => None,
    }
}

fn reset_after_summary(m: &mut UpdaterModel) {
    m.state = UpdaterState::Main;
    m.checked.clear();
    m.total_update = 0;
    m.updating_remaining = 0;

    for item in m.items.iter_mut() {
        if item.status == ToolStatus::Updated || item.status == ToolStatus::Failed {
            item.message.clear();
            if item.local_version == "MISSING" {
                item.status = ToolStatus::Missing;
            } else if item.remote_version != "..."
                && item.remote_version != "Checking..."
                && item.remote_version != "Unknown"
                && item.remote_version != item.local_version
            {
                item.status = ToolStatus::Outdated;
            } else {
                item.status = ToolStatus::Installed;
            }
        }
    }
}

fn handle_main(app: &mut App, key: KeyEvent) -> Option<Action> {
    let m = &mut app.updater;
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
            Some(Action::Quit)
        }
        KeyCode::Esc => {
            if !m.search_query.is_empty() {
                m.search_query.clear();
                m.filtered_indices = None;
                None
            } else {
                app.should_quit = true;
                Some(Action::Quit)
            }
        }
        KeyCode::Char('/') => {
            m.state = UpdaterState::Search;
            m.search_query.clear();
            None
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if m.cursor > 0 {
                m.cursor -= 1;
                while !m.is_item_visible(m.cursor) && m.cursor > 0 {
                    m.cursor -= 1;
                }
            }
            None
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if m.cursor < m.items.len() - 1 {
                m.cursor += 1;
                while !m.is_item_visible(m.cursor) && m.cursor < m.items.len() - 1 {
                    m.cursor += 1;
                }
            }
            None
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            m.jump_to_category(Category::Code);
            None
        }
        KeyCode::Char('t') | KeyCode::Char('T') => {
            m.jump_to_category(Category::Term);
            None
        }
        KeyCode::Char('i') | KeyCode::Char('I') => {
            m.jump_to_category(Category::Ide);
            None
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            m.jump_to_category(Category::Prod);
            None
        }
        KeyCode::Char('f') | KeyCode::Char('F') => {
            m.jump_to_category(Category::Infra);
            None
        }
        KeyCode::Char('u') | KeyCode::Char('U') => {
            m.jump_to_category(Category::Utils);
            None
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            m.jump_to_category(Category::Runtime);
            None
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            m.jump_to_category(Category::Sys);
            None
        }
        KeyCode::Char(' ') => {
            if m.checked.contains(&m.cursor) {
                m.checked.remove(&m.cursor);
            } else {
                m.checked.insert(m.cursor);
            }
            None
        }
        KeyCode::Char('g') | KeyCode::Char('G') | KeyCode::Char('a') | KeyCode::Char('A') => {
            toggle_category_selection(m);
            None
        }
        KeyCode::Char('d') | KeyCode::Char('D') => {
            if m.loading_count > 0 {
                return None;
            }
            if m.checked.is_empty() {
                m.checked.insert(m.cursor);
            }
            m.state = UpdaterState::Preview;
            None
        }
        KeyCode::Enter => {
            if m.loading_count > 0 {
                return None;
            }
            if m.checked.is_empty() {
                m.checked.insert(m.cursor);
            }
            if m.has_critical_selected() {
                m.state = UpdaterState::Confirm;
                None
            } else {
                m.state = UpdaterState::Updating;
                start_updates(m)
            }
        }
        _ => None,
    }
}

fn toggle_category_selection(m: &mut UpdaterModel) {
    let current_cat = m.items[m.cursor].tool.category;
    let all_selected = m
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| item.tool.category == current_cat)
        .all(|(i, _)| m.checked.contains(&i));

    for (i, item) in m.items.iter().enumerate() {
        if item.tool.category == current_cat {
            if all_selected {
                m.checked.remove(&i);
            } else {
                m.checked.insert(i);
            }
        }
    }
}

fn start_updates(m: &mut UpdaterModel) -> Option<Action> {
    m.build_update_queue();
    if m.update_queue.is_empty() {
        m.state = UpdaterState::Summary;
        return None;
    }

    if let Some(first) = m.update_queue.pop_front() {
        m.current_update = Some(first);
        m.current_log = UpdaterModel::get_update_log_text(&m.items[first].tool);
        return Some(Action::StartUpdate(first));
    }
    None
}
