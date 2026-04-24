//! Scanner tab state: repos table + grouping + scan progress, and the
//! sibling Port scanner state (small, scanner-adjacent).

use super::{ScannerState, SortField};
use crate::scanner::port_scanner::PortInfo;
use crate::scanner::repo_scanner::{DiscoveredDir, RepoInfo};
use std::collections::HashSet;

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
    /// Cached child repos when viewing a container detail.
    pub container_children: Vec<RepoInfo>,
    pub container_cursor: usize,
    pub container_sort: u8,
    /// Ordered list of group names (cached, rebuilt after scan/sort).
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

    /// Rebuild group_order from current repos. Call after scan or sort.
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
            if ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });
        self.rebuild_group_order();
    }
}

pub struct PortScannerModel {
    pub ports: Vec<PortInfo>,
    /// Visual display order: indices into `ports`, built after scan.
    pub display_order: Vec<usize>,
    /// Cursor position in display_order (not in ports).
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

    /// Resolve cursor position to the underlying port index.
    pub fn cursor_port_index(&self) -> Option<usize> {
        self.display_order.get(self.cursor).copied()
    }
}
