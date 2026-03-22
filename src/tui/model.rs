use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;

use crate::config::SparkConfig;
use crate::core::types::*;
use crate::core::inventory::get_inventory;
use crate::scanner::repo_scanner::RepoInfo;
use crate::scanner::port_scanner::PortInfo;
use crate::scanner::repo_manager::ManagedRepo;

/// Top-level application mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppMode {
    Updater,
    Scanner,
}

/// Updater state machine
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdaterState {
    Splash,
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
    Scanning,
    ScanResults,
    RepoDetail,
    CleanConfirm,
    Cleaning,
    CleanSummary,
    PortScan,
    PortKillConfirm,
    RepoManager,
    RepoCloneInput,
    RepoCloneSummary,
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

    // Discovery
    DiscoveredDirs {
        dirs: Vec<PathBuf>,
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
            state: UpdaterState::Splash,
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
    pub search_query: String,
    pub scan_progress_repos: usize,
    pub scan_progress_dirs: usize,
    pub scan_progress_current: String,
    pub clean_results: Vec<(usize, u64, bool, Option<String>)>,
    pub total_recoverable: u64,
    pub discovered_dirs: Vec<PathBuf>,
    pub selected_scan_dirs: HashSet<usize>,
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
            search_query: String::new(),
            scan_progress_repos: 0,
            scan_progress_dirs: 0,
            scan_progress_current: String::new(),
            clean_results: Vec::new(),
            total_recoverable: 0,
            discovered_dirs: Vec::new(),
            selected_scan_dirs: HashSet::new(),
        }
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
    }
}

/// Port scanner model: tracks discovered ports and kill selections
pub struct PortScannerModel {
    pub ports: Vec<PortInfo>,
    pub cursor: usize,
    pub checked: HashSet<usize>,
}

impl PortScannerModel {
    pub fn new() -> Self {
        Self {
            ports: Vec::new(),
            cursor: 0,
            checked: HashSet::new(),
        }
    }
}

/// Repo manager model: tracks managed repos, clone input, and pull operations
pub struct RepoManagerModel {
    pub repos: Vec<ManagedRepo>,
    pub cursor: usize,
    pub checked: HashSet<usize>,
    pub root: PathBuf,
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
    pub fn new(repos_root: &std::path::Path) -> Self {
        Self {
            repos: Vec::new(),
            cursor: 0,
            checked: HashSet::new(),
            root: repos_root.to_path_buf(),
            clone_input: String::new(),
            clone_error: None,
            cloning: false,
            last_clone: None,
        }
    }
}

/// Top-level application state: holds both mode models, config, and display info
pub struct App {
    pub mode: AppMode,
    pub updater: UpdaterModel,
    pub scanner: ScannerModel,
    pub port_scanner: PortScannerModel,
    pub repo_manager: RepoManagerModel,
    pub config: SparkConfig,
    pub should_quit: bool,
    pub dry_run: bool,
    pub width: u16,
    pub height: u16,
    pub tick_count: usize,
}

impl App {
    pub fn new(config: SparkConfig) -> Self {
        Self {
            mode: AppMode::Updater,
            updater: UpdaterModel::new(),
            scanner: ScannerModel::new(),
            port_scanner: PortScannerModel::new(),
            repo_manager: RepoManagerModel::new(&config.repos_root),
            config,
            should_quit: false,
            dry_run: false,
            width: 0,
            height: 0,
            tick_count: 0,
        }
    }
}
