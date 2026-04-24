//! Event dispatch for the TUI: keys → Action, messages → state updates.
//!
//! Splits:
//! - mod.rs          — Action enum, global key dispatcher, welcome screen
//! - messages.rs     — handle_message (background task → state)
//! - updater_keys.rs — handle_updater_key (Updater tab key bindings)
//!
//! Non-updater tabs are handled by `tui::scanner_keys`.

mod messages;
mod updater_keys;

pub use messages::handle_message;
use updater_keys::handle_updater_key;

use crate::tui::model::*;
use crate::tui::scanner_keys;
use crate::utils::shell::debug_log;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Side-effect actions dispatched from key/message handlers to the event loop.
pub enum Action {
    /// Exit the application.
    Quit,
    /// Trigger remote version checks after cache warmup.
    StartVersionChecks,
    /// Begin updating a specific tool by index.
    StartUpdate(usize),
    /// Start scanning the given directories for repos.
    StartScan(Vec<std::path::PathBuf>),
    /// Discover project directories in home folder.
    DiscoverDirs,
    /// Delete artifact directories (node_modules, target/, etc.).
    CleanArtifacts(Vec<std::path::PathBuf>),
    /// Move a repository to trash.
    TrashRepo(std::path::PathBuf),
    /// Scan for listening ports.
    ScanPorts,
    /// Kill processes by PID list.
    KillProcesses(Vec<u32>),
    /// List managed repos.
    ListManagedRepos,
    /// Check status of managed repos (fetch + compare).
    CheckRepoStatuses,
    /// Pull specific repos by index.
    PullRepos(Vec<usize>),
    /// Clone a repo from URL into managed root.
    CloneRepo(String),
    /// Open a directory in the system file manager / terminal.
    OpenDir(std::path::PathBuf),
    /// Load container children in background.
    LoadContainerChildren(std::path::PathBuf),
    /// Scan system for cleanable items (Docker, caches, logs).
    ScanSystem,
    /// Clean a specific system item by index.
    CleanSystemItem(usize),
    CleanSystemItems(Vec<usize>),
    /// Start security audit on a path.
    StartAudit(std::path::PathBuf),
}

pub fn handle_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return Some(Action::Quit);
    }

    if app.show_welcome {
        return handle_welcome_key(app, key);
    }

    debug_log(&format!(
        "KEY: {:?} | mode={:?} scanner_state={:?}",
        key.code, app.mode, app.scanner.state
    ));

    let in_text_input = app.mode == AppMode::Scanner
        && matches!(
            app.scanner.state,
            ScannerState::RepoCloneInput | ScannerState::ScanAddPath
        );
    let in_search = app.mode == AppMode::Updater && app.updater.state == UpdaterState::Search;
    let in_blocking = (app.mode == AppMode::Updater && app.updater.state == UpdaterState::Updating)
        || (app.mode == AppMode::Scanner && app.scanner.state == ScannerState::Scanning)
        || (app.mode == AppMode::Scanner && app.scanner.state == ScannerState::Cleaning)
        || (app.mode == AppMode::Scanner && app.scanner.state == ScannerState::ContainerLoading);

    if key.code == KeyCode::Tab && !in_text_input && !in_search && !in_blocking {
        if let Some(action) = cycle_tab(app) {
            return action;
        }
    }

    match app.mode {
        AppMode::Updater => handle_updater_key(app, key),
        AppMode::Scanner => scanner_keys::handle_scanner_key(app, key),
    }
}

/// Tab cycle: Scanner → Repos → Ports → System → Audit → Updater → Scanner.
/// Returns Some(None) to indicate the tab cycled but no side effect was triggered,
/// or Some(Some(action)) when a scan/list/etc. needs to fire.
fn cycle_tab(app: &mut App) -> Option<Option<Action>> {
    if app.mode == AppMode::Scanner
        && matches!(
            app.scanner.state,
            ScannerState::ScanConfig | ScannerState::ScanResults | ScannerState::RepoDetail
        )
    {
        app.scanner.state = ScannerState::RepoManager;
        return Some(Some(Action::ListManagedRepos));
    }
    if app.mode == AppMode::Scanner
        && matches!(
            app.scanner.state,
            ScannerState::RepoManager | ScannerState::RepoCloneSummary
        )
    {
        app.scanner.state = ScannerState::PortScan;
        return Some(Some(Action::ScanPorts));
    }
    if app.mode == AppMode::Scanner && matches!(app.scanner.state, ScannerState::PortScan) {
        app.scanner.state = ScannerState::SystemClean;
        return Some(Some(Action::ScanSystem));
    }
    if app.mode == AppMode::Scanner && matches!(app.scanner.state, ScannerState::SystemClean) {
        app.scanner.state = ScannerState::SecretAudit;
        if app.audit.results.is_empty() {
            let path = std::env::current_dir().unwrap_or_else(|_| app.config.repos_root.clone());
            return Some(Some(Action::StartAudit(path)));
        }
        return Some(None);
    }
    if app.mode == AppMode::Scanner && matches!(app.scanner.state, ScannerState::SecretAudit) {
        app.mode = AppMode::Updater;
        return Some(None);
    }
    if app.mode == AppMode::Updater {
        app.mode = AppMode::Scanner;
        if !app.scanner.repos.is_empty() {
            app.scanner.state = ScannerState::ScanResults;
        } else {
            app.scanner.state = ScannerState::ScanConfig;
            if app.scanner.discovered_dirs.is_empty() {
                return Some(Some(Action::DiscoverDirs));
            }
        }
        return Some(None);
    }
    None
}

fn handle_welcome_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
            Some(Action::Quit)
        }
        KeyCode::Char('s') | KeyCode::Char('S') | KeyCode::Enter => {
            app.show_welcome = false;
            app.mode = AppMode::Scanner;
            if app.scanner.discovered_dirs.is_empty() {
                return Some(Action::DiscoverDirs);
            }
            None
        }
        KeyCode::Char('c') | KeyCode::Char('C') => {
            app.show_welcome = false;
            app.mode = AppMode::Scanner;
            app.scanner.state = ScannerState::SystemClean;
            Some(Action::ScanSystem)
        }
        KeyCode::Char('u') | KeyCode::Char('U') => {
            app.show_welcome = false;
            app.mode = AppMode::Updater;
            None
        }
        KeyCode::Char('p') | KeyCode::Char('P') => {
            app.show_welcome = false;
            app.mode = AppMode::Scanner;
            app.scanner.state = ScannerState::PortScan;
            Some(Action::ScanPorts)
        }
        KeyCode::Char('g') | KeyCode::Char('G') => {
            app.show_welcome = false;
            app.mode = AppMode::Scanner;
            app.scanner.state = ScannerState::RepoManager;
            Some(Action::ListManagedRepos)
        }
        _ => None,
    }
}
