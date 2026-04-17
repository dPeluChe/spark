use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

use crate::config::SparkConfig;
use crate::core::types::*;
use crate::core::inventory::get_inventory;
use crate::scanner::repo_scanner::{RepoInfo, DiscoveredDir};
use crate::scanner::port_scanner::PortInfo;
use crate::scanner::repo_manager::ManagedRepo;
use crate::scanner::system_cleaner::{CleanableItem, CleanCategory};
use crate::scanner::secret_scanner::AuditResult;
use crate::scanner::dep_scanner::DepVulnerability;

/// Top-level application mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Updater,
    Scanner,
}

/// Updater state machine
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdaterState {
    Main,
    Search,
    Preview,
    Confirm,
    Updating,
    Summary,
}

/// Scanner state machine
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

/// Sort field for scanner results
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortField {
    Name,
    LastCommit,
    Size,
    Health,
    ArtifactSize,
}

/// Messages from background tasks to the TUI
#[derive(Debug)]
pub enum AppMessage {
    // Updater messages
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

    // Scanner messages
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
        /// On success, the cloned repo path for summary
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

/// Updater mode model: tracks tool states, selection, search, and update queue
pub struct UpdaterModel {
    pub state: UpdaterState,
    pub items: Vec<ToolState>,
    pub cursor: usize,
    pub checked: HashSet<usize>,
    pub loading_count: usize,
    pub update_queue: VecDeque<usize>,
    pub current_update: Option<usize>,
    pub current_log: String,
    pub total_update: usize,
    pub updating_remaining: usize,
    pub search_query: String,
    pub filtered_indices: Option<Vec<usize>>,
    pub splash_frame: usize,
}

impl UpdaterModel {
    pub fn new() -> Self {
        let inv = get_inventory();
        let items: Vec<ToolState> = inv
            .into_iter()
            .map(|t| ToolState {
                tool: t,
                status: ToolStatus::Checking,
                local_version: "...".into(),
                remote_version: "...".into(),
                message: String::new(),
            })
            .collect();
        let loading_count = items.len();

        Self {
            state: UpdaterState::Main,
            items,
            cursor: 0,
            checked: HashSet::new(),
            loading_count,
            update_queue: VecDeque::new(),
            current_update: None,
            current_log: String::new(),
            total_update: 0,
            updating_remaining: 0,
            search_query: String::new(),
            filtered_indices: None,
            splash_frame: 0,
        }
    }

    /// Check if an item index is visible (passes current filter)
    pub fn is_item_visible(&self, index: usize) -> bool {
        match &self.filtered_indices {
            None => true,
            Some(indices) => indices.contains(&index),
        }
    }

    /// Update the search filter
    pub fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_indices = None;
            return;
        }

        let query = self.search_query.to_lowercase();
        let indices: Vec<usize> = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.tool.name.to_lowercase().contains(&query)
                    || item.tool.binary.to_lowercase().contains(&query)
                    || item.tool.package.to_lowercase().contains(&query)
                    || item.tool.category.label().to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();

        if let Some(first) = indices.first() {
            self.cursor = *first;
        }
        self.filtered_indices = Some(indices);
    }

    /// Jump cursor to first item in a category
    pub fn jump_to_category(&mut self, cat: Category) {
        for (i, item) in self.items.iter().enumerate() {
            if item.tool.category == cat {
                self.cursor = i;
                return;
            }
        }
    }

    /// Check if any selected item is a Runtime
    pub fn has_critical_selected(&self) -> bool {
        self.checked
            .iter()
            .any(|&i| self.items[i].tool.category == Category::Runtime)
    }

    /// Build update queue from checked items
    pub fn build_update_queue(&mut self) {
        self.update_queue.clear();
        self.total_update = 0;
        self.updating_remaining = 0;
        self.current_update = None;

        for i in 0..self.items.len() {
            if self.checked.contains(&i) {
                self.items[i].status = ToolStatus::Updating;
                self.update_queue.push_back(i);
                self.total_update += 1;
                self.updating_remaining += 1;
            }
        }
    }

    /// Get the command log text for the current update
    pub fn get_update_log_text(tool: &Tool) -> String {
        match tool.method {
            UpdateMethod::BrewPkg => {
                format!("> brew upgrade {}", tool.package)
            }
            UpdateMethod::NpmPkg | UpdateMethod::NpmSys | UpdateMethod::Claude => {
                format!("> npm install -g {}@latest", tool.package)
            }
            UpdateMethod::Omz => "> $ZSH/tools/upgrade.sh".into(),
            UpdateMethod::Toad => "> curl -fsSL batrachian.ai/install | sh".into(),
            UpdateMethod::MacApp => format!("> brew upgrade --cask {}", tool.package),
            _ => format!("> Updating {}...", tool.name),
        }
    }
}

/// Scanner mode model: tracks repos, scan progress, sorting, and clean results
pub struct ScannerModel {
    pub state: ScannerState,
    pub repos: Vec<RepoInfo>,
    pub cursor: usize,
    pub checked: HashSet<usize>,
    pub sort_by: SortField,
    pub sort_ascending: bool,
    pub scan_progress_repos: usize,
    pub scan_progress_dirs: usize,
    pub scan_progress_current: String,
    pub clean_results: Vec<(usize, u64, bool, Option<String>)>,
    pub total_recoverable: u64,
    pub discovered_dirs: Vec<DiscoveredDir>,
    pub selected_scan_dirs: HashSet<usize>,
    pub path_input: String,
    /// Cached child repos when viewing a container detail
    pub container_children: Vec<RepoInfo>,
    pub container_cursor: usize,
    pub container_sort: u8,
    /// Ordered list of group names (cached, rebuilt after scan/sort)
    pub group_order: Vec<String>,
}

impl ScannerModel {
    pub fn new() -> Self {
        Self {
            state: ScannerState::ScanConfig,
            repos: Vec::new(),
            cursor: 0,
            checked: HashSet::new(),
            sort_by: SortField::Health,
            sort_ascending: true,
            scan_progress_repos: 0,
            scan_progress_dirs: 0,
            scan_progress_current: String::new(),
            clean_results: Vec::new(),
            total_recoverable: 0,
            discovered_dirs: Vec::new(),
            selected_scan_dirs: HashSet::new(),
            path_input: String::new(),
            container_children: Vec::new(),
            container_cursor: 0,
            container_sort: 0,
            group_order: Vec::new(),
        }
    }

    /// Rebuild the cached group order from current repos (call after scan or sort).
    pub fn rebuild_group_order(&mut self) {
        let mut seen = HashSet::new();
        let mut order = Vec::new();
        for repo in &self.repos {
            if seen.insert(repo.group.as_str()) {
                order.push(repo.group.clone());
            }
        }
        self.group_order = order;
    }

    pub fn sort_repos(&mut self) {
        let ascending = self.sort_ascending;
        self.repos.sort_by(|a, b| {
            let cmp = match self.sort_by {
                SortField::Name => a.name.cmp(&b.name),
                SortField::LastCommit => a.last_commit_date.cmp(&b.last_commit_date),
                SortField::Size => a.total_size.cmp(&b.total_size),
                SortField::Health => a.health_score.cmp(&b.health_score),
                SortField::ArtifactSize => a.artifact_size.cmp(&b.artifact_size),
            };
            if ascending { cmp } else { cmp.reverse() }
        });
        self.rebuild_group_order();
    }
}

/// Port scanner model: tracks discovered ports and kill selections
pub struct PortScannerModel {
    pub ports: Vec<PortInfo>,
    /// Visual display order: indices into `ports`, built after scan
    pub display_order: Vec<usize>,
    /// Cursor position in display_order (not in ports)
    pub cursor: usize,
    pub checked: HashSet<usize>,
    pub scanning: bool,
}

impl PortScannerModel {
    pub fn new() -> Self {
        Self {
            ports: Vec::new(),
            display_order: Vec::new(),
            cursor: 0,
            checked: HashSet::new(),
            scanning: false,
        }
    }

    /// Get the actual port index for the current cursor position
    pub fn cursor_port_index(&self) -> Option<usize> {
        self.display_order.get(self.cursor).copied()
    }
}

/// Repo manager model: tracks managed repos, clone input, and pull operations
pub struct RepoManagerModel {
    pub repos: Vec<ManagedRepo>,
    pub cursor: usize,
    pub checked: HashSet<usize>,
    pub root: PathBuf,
    pub root_source: &'static str,
    pub clone_input: String,
    pub clone_error: Option<String>,
    pub cloning: bool,
    /// Last clone result for summary display
    pub last_clone: Option<CloneSummary>,
}

/// Summary info shown after a successful clone
pub struct CloneSummary {
    pub repo_path: String,
    pub repo_name: String,
    pub remote_url: String,
    pub alias_cmd: String,
    pub short_path: String,
}

impl RepoManagerModel {
    pub fn new(repos_root: &std::path::Path, root_source: &'static str) -> Self {
        Self {
            repos: Vec::new(),
            cursor: 0,
            checked: HashSet::new(),
            root: repos_root.to_path_buf(),
            root_source,
            clone_input: String::new(),
            clone_error: None,
            cloning: false,
            last_clone: None,
        }
    }
}

/// System cleaner model: Docker, caches, logs
pub struct SystemCleanerModel {
    pub items: Vec<CleanableItem>,
    /// Current item index (indexes into `items`)
    pub cursor: usize,
    pub checked: std::collections::HashSet<usize>,
    pub scanning: bool,
    /// Display rows: None = category header (non-selectable), Some(i) = items[i]
    pub display_order: Vec<Option<usize>>,
}

impl SystemCleanerModel {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            cursor: 0,
            checked: std::collections::HashSet::new(),
            scanning: false,
            display_order: Vec::new(),
        }
    }

    /// Rebuild display_order after items change (includes category header sentinels)
    pub fn rebuild_display_order(&mut self) {
        let categories = [CleanCategory::Docker, CleanCategory::VMs, CleanCategory::Cache, CleanCategory::Logs, CleanCategory::Downloads];
        let mut order = Vec::new();
        for cat in &categories {
            let cat_indices: Vec<usize> = self.items.iter().enumerate()
                .filter(|(_, i)| i.category == *cat)
                .map(|(idx, _)| idx)
                .collect();
            if cat_indices.is_empty() { continue; }
            order.push(None); // category header row
            for idx in cat_indices { order.push(Some(idx)); }
        }
        self.display_order = order;
        // Clamp cursor to valid item
        if self.cursor >= self.items.len() && !self.items.is_empty() {
            self.cursor = self.items.len() - 1;
        }
    }

    /// Move selection up, skipping header rows
    pub fn move_up(&mut self) {
        let dc = self.display_order.iter().position(|d| *d == Some(self.cursor)).unwrap_or(0);
        let mut i = dc;
        loop {
            if i == 0 { break; }
            i -= 1;
            if let Some(Some(idx)) = self.display_order.get(i) {
                self.cursor = *idx;
                break;
            }
        }
    }

    /// Move selection down, skipping header rows
    pub fn move_down(&mut self) {
        let dc = self.display_order.iter().position(|d| *d == Some(self.cursor)).unwrap_or(0);
        let mut i = dc + 1;
        while i < self.display_order.len() {
            if let Some(Some(idx)) = self.display_order.get(i) {
                self.cursor = *idx;
                break;
            }
            i += 1;
        }
    }
}

/// Security audit model
pub struct AuditModel {
    pub results: Vec<AuditResult>,
    pub cursor: usize,
    pub detail_cursor: usize,
    pub scanning: bool,
    pub total_critical: usize,
    pub total_warning: usize,
    pub total_info: usize,
    pub scan_path: Option<std::path::PathBuf>,
    pub path_input: String,
    /// Dependency vulnerabilities from OSV.dev scan
    pub dep_vulns: Vec<DepVulnerability>,
    /// Cursor in the dep vulns detail view
    pub dep_cursor: usize,
}

impl AuditModel {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            cursor: 0,
            detail_cursor: 0,
            scanning: false,
            total_critical: 0,
            total_warning: 0,
            total_info: 0,
            scan_path: None,
            path_input: String::new(),
            dep_vulns: Vec::new(),
            dep_cursor: 0,
        }
    }
}

/// A toast notification shown briefly at the bottom of the screen
pub struct Toast {
    pub message: String,
    pub is_error: bool,
    /// Tick when the toast was created (auto-dismiss after N ticks)
    pub created_at: usize,
}

/// Top-level application state: holds both mode models, config, and display info
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
        assert!(m.items[indices[0]].tool.name.to_lowercase().contains("claude"));
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

        // Find a Runtime tool and select it
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
