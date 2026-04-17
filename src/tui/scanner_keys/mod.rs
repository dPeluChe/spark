//! Key event handlers for all Scanner-mode tabs.
//! Dispatches to sub-modules by scanner state group.

mod scanner_tab;
mod repo_tab;
mod port_tab;
mod system_tab;
mod audit_tab;

use crossterm::event::KeyEvent;
use crate::tui::model::*;
use crate::tui::update::Action;

pub const PAGE_JUMP: usize = 20;

/// Handle key events for Scanner mode — dispatches to the active tab
pub fn handle_scanner_key(app: &mut App, key: KeyEvent) -> Option<Action> {
    match app.scanner.state {
        // Scanner tab (scan config, results, detail, clean, delete)
        ScannerState::ScanConfig | ScannerState::ScanAddPath
        | ScannerState::ContainerLoading | ScannerState::Scanning
        | ScannerState::ScanResults | ScannerState::RepoDetail
        | ScannerState::ContainerChildDetail | ScannerState::ContainerChildDelete
        | ScannerState::HealthHelp | ScannerState::CleanConfirm
        | ScannerState::DeleteRepoConfirm | ScannerState::Cleaning
            => scanner_tab::handle(app, key),

        // Port tab
        ScannerState::PortScan | ScannerState::PortAction | ScannerState::PortKillConfirm
            => port_tab::handle(app, key),

        // System tab
        ScannerState::SystemClean | ScannerState::SystemCleanConfirm
            => system_tab::handle(app, key),

        // Repo manager tab
        ScannerState::RepoManager | ScannerState::RepoAction
        | ScannerState::RepoCloneInput | ScannerState::RepoCloneSummary
            => repo_tab::handle(app, key),

        // Audit tab
        ScannerState::SecretAudit | ScannerState::SecretAuditScanning
        | ScannerState::SecretAuditDetail | ScannerState::SecretAuditDeps
            => audit_tab::handle(app, key),
    }
}
