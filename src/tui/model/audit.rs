//! Security audit tab state: scan results + dep vulns.

use crate::scanner::dep_scanner::DepVulnerability;
use crate::scanner::secret_scanner::AuditResult;

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
    /// Dependency vulnerabilities from OSV.dev scan.
    pub dep_vulns: Vec<DepVulnerability>,
    /// Cursor in the dep vulns detail view.
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
