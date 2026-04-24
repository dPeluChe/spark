//! TUI state model — enums, per-tab submodels, and the top-level `App` struct.
//!
//! Submodules split by tab:
//! - updater.rs — UpdaterModel
//! - scanner.rs — ScannerModel + PortScannerModel
//! - repo.rs    — RepoManagerModel + CloneSummary
//! - system.rs  — SystemCleanerModel
//! - audit.rs   — AuditModel

mod audit;
mod repo;
mod scanner;
mod system;
mod updater;

pub use audit::AuditModel;
pub use repo::{CloneSummary, RepoManagerModel};
pub use scanner::{PortScannerModel, ScannerModel};
pub use system::SystemCleanerModel;
pub use updater::UpdaterModel;

use crate::config::SparkConfig;
use crate::core::types::*;
use crate::scanner::dep_scanner::DepVulnerability;
use crate::scanner::port_scanner::PortInfo;
use crate::scanner::repo_manager::ManagedRepo;
use crate::scanner::repo_scanner::{DiscoveredDir, RepoInfo};
use crate::scanner::secret_scanner::AuditResult;
use crate::scanner::system_cleaner::CleanableItem;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Updater,
    Scanner,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdaterState {
    Main,
    Search,
    Preview,
    Confirm,
    Updating,
    Summary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScannerState {
    ScanConfig,
    ScanAddPath,
    Scanning,
    ContainerLoading,
    ScanResults,
    RepoDetail,
    ContainerChildDetail,
    ContainerChildDelete,
    CleanConfirm,
    HealthHelp,
    DeleteRepoConfirm,
    Cleaning,
    PortScan,
    PortAction,
    PortKillConfirm,
    SystemClean,
    SystemCleanConfirmBulk,
    #[allow(dead_code)]
    SystemCleanConfirm,
    RepoManager,
    RepoAction,
    RepoCloneInput,
    RepoCloneSummary,
    SecretAudit,
    SecretAuditScanning,
    SecretAuditDetail,
    SecretAuditDeps,
    SecretAuditPathInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortField {
    Name,
    LastCommit,
    Size,
    Health,
    ArtifactSize,
}

/// Messages from background tasks to the TUI event loop.
#[derive(Debug)]
pub enum AppMessage {
    // Updater
    CheckResult {
        index: usize,
        local_version: String,
        remote_version: String,
        status: ToolStatus,
        message: String,
    },
    WarmUpFinished,
    UpdateResult {
        index: usize,
        success: bool,
        message: String,
        new_version: String,
    },

    // Scanner
    ScanProgress {
        repos_found: usize,
        dirs_scanned: usize,
        current_dir: String,
    },
    ScanComplete {
        repos: Vec<RepoInfo>,
    },
    CleanResult {
        index: usize,
        bytes_recovered: u64,
        success: bool,
        error: Option<String>,
    },
    CleanAllComplete,

    // Port scanner
    PortScanResult {
        ports: Vec<PortInfo>,
    },
    KillResult {
        pid: u32,
        success: bool,
        error: Option<String>,
    },

    // Repo manager
    RepoListResult {
        repos: Vec<ManagedRepo>,
    },
    RepoStatusResult {
        index: usize,
        status: crate::scanner::repo_manager::RepoStatus,
    },
    RepoPullResult {
        index: usize,
        success: bool,
        message: String,
    },
    CloneResult {
        success: bool,
        message: String,
        /// On success, the cloned repo path for summary.
        clone_path: Option<String>,
    },

    // System cleaner
    SystemScanResult {
        items: Vec<CleanableItem>,
    },
    SystemCleanItemResult {
        index: usize,
        recovered: u64,
        success: bool,
        error: Option<String>,
    },

    // Container children
    ContainerChildrenResult {
        children: Vec<RepoInfo>,
    },

    // Discovery
    DiscoveredDirs {
        dirs: Vec<DiscoveredDir>,
    },

    // Security audit
    AuditScanResult {
        results: Vec<AuditResult>,
        dep_vulns: Vec<DepVulnerability>,
    },
}

/// A toast notification shown briefly at the bottom of the screen.
pub struct Toast {
    pub message: String,
    pub is_error: bool,
    /// Tick when the toast was created (auto-dismiss after N ticks).
    pub created_at: usize,
}

/// Top-level application state: both mode models, config, and display info.
pub struct App {
    pub mode: AppMode,
    pub show_welcome: bool,
    pub updater: UpdaterModel,
    pub scanner: ScannerModel,
    pub port_scanner: PortScannerModel,
    pub system_cleaner: SystemCleanerModel,
    pub repo_manager: RepoManagerModel,
    pub audit: AuditModel,
    pub config: SparkConfig,
    pub should_quit: bool,
    pub dry_run: bool,
    pub width: u16,
    pub height: u16,
    pub tick_count: usize,
    pub toast: Option<Toast>,
}

impl App {
    pub fn new(config: SparkConfig) -> Self {
        let config_exists = dirs::config_dir()
            .unwrap_or_default()
            .join("spark")
            .join("config.toml")
            .exists();
        let root_source: &'static str = if config_exists {
            "configured"
        } else if crate::config::detect_ghq_root().is_some() {
            "ghq"
        } else {
            "default"
        };
        Self {
            mode: AppMode::Scanner,
            show_welcome: true,
            updater: UpdaterModel::new(),
            scanner: ScannerModel::new(),
            port_scanner: PortScannerModel::new(),
            system_cleaner: SystemCleanerModel::new(),
            repo_manager: RepoManagerModel::new(&config.repos_root, root_source),
            audit: AuditModel::new(),
            config,
            should_quit: false,
            dry_run: false,
            width: 0,
            height: 0,
            tick_count: 0,
            toast: None,
        }
    }

    pub fn show_toast(&mut self, message: String, is_error: bool) {
        self.toast = Some(Toast {
            message,
            is_error,
            created_at: self.tick_count,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_new_defaults() {
        let app = App::new(SparkConfig::default());
        assert_eq!(app.mode, AppMode::Scanner);
        assert!(app.show_welcome);
        assert!(!app.should_quit);
        assert!(!app.dry_run);
    }

    #[test]
    fn test_updater_model_initializes_with_tools() {
        let m = UpdaterModel::new();
        assert!(!m.items.is_empty());
        assert_eq!(m.state, UpdaterState::Main);
        assert_eq!(m.cursor, 0);
        assert!(m.checked.is_empty());
        assert_eq!(m.loading_count, m.items.len());
    }

    #[test]
    fn test_updater_search_filter() {
        let mut m = UpdaterModel::new();
        m.search_query = "claude".into();
        m.update_filter();
        assert!(m.filtered_indices.is_some());
        let indices = m.filtered_indices.as_ref().unwrap();
        assert!(!indices.is_empty());
        assert!(m.items[indices[0]]
            .tool
            .name
            .to_lowercase()
            .contains("claude"));
    }

    #[test]
    fn test_updater_clear_search() {
        let mut m = UpdaterModel::new();
        m.search_query = "claude".into();
        m.update_filter();
        assert!(m.filtered_indices.is_some());

        m.search_query.clear();
        m.update_filter();
        assert!(m.filtered_indices.is_none());
    }

    #[test]
    fn test_updater_is_item_visible_no_filter() {
        let m = UpdaterModel::new();
        assert!(m.is_item_visible(0));
        assert!(m.is_item_visible(m.items.len() - 1));
    }

    #[test]
    fn test_updater_jump_to_category() {
        let mut m = UpdaterModel::new();
        m.jump_to_category(Category::Runtime);
        assert_eq!(m.items[m.cursor].tool.category, Category::Runtime);
    }

    #[test]
    fn test_updater_has_critical_selected() {
        let mut m = UpdaterModel::new();
        assert!(!m.has_critical_selected());

        for (i, item) in m.items.iter().enumerate() {
            if item.tool.category == Category::Runtime {
                m.checked.insert(i);
                break;
            }
        }
        assert!(m.has_critical_selected());
    }

    #[test]
    fn test_updater_build_update_queue() {
        let mut m = UpdaterModel::new();
        m.checked.insert(0);
        m.checked.insert(1);
        m.build_update_queue();
        assert_eq!(m.total_update, 2);
        assert_eq!(m.updating_remaining, 2);
        assert_eq!(m.update_queue.len(), 2);
    }

    #[test]
    fn test_scanner_model_defaults() {
        let s = ScannerModel::new();
        assert_eq!(s.state, ScannerState::ScanConfig);
        assert!(s.repos.is_empty());
        assert_eq!(s.sort_by, SortField::Health);
    }

    #[test]
    fn test_port_scanner_model_defaults() {
        let p = PortScannerModel::new();
        assert!(p.ports.is_empty());
        assert_eq!(p.cursor, 0);
    }
}
