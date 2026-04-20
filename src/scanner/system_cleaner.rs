//! System cleanup: Docker resources, dev caches, and dev logs.
//!
//! Safety model inspired by tw93/mole:
//! - Path validation against protected system paths
//! - App-aware: skips caches of running apps
//! - Age-based: only cleans logs older than threshold
//! - Operation logging to ~/.config/spark/operations.log
//! - Dry-run support
//! - Whitelist support via ~/.config/spark/whitelist.txt

use std::path::{Path, PathBuf};
use std::process::Command;

pub(crate) const LOG_AGE_DAYS: u64 = 7;

/// Protected paths that must NEVER be deleted
const PROTECTED_PATHS: &[&str] = &[
    "/",
    "/System",
    "/bin",
    "/sbin",
    "/usr",
    "/etc",
    "/var/db",
    "/Library/Extensions",
    "/private/var/db",
    "/Applications",
    "/Library",
];

/// A cleanable system resource
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CleanableItem {
    pub category: CleanCategory,
    pub name: String,
    pub detail: String,
    pub size: u64,
    pub clean_cmd: CleanCommand,
    /// Whether the owning app is currently running
    pub app_running: bool,
    /// Age of the item in days (for logs)
    pub age_days: Option<u64>,
    /// Risk level of cleaning this item
    pub risk: CleanRisk,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanRisk {
    Safe,    // Caches — can be rebuilt automatically
    Caution, // Large items — may need rebuild time
    Danger,  // Running apps or system-level
}

impl std::fmt::Display for CleanRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CleanRisk::Safe => write!(f, "safe"),
            CleanRisk::Caution => write!(f, "caution"),
            CleanRisk::Danger => write!(f, "danger"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CleanCategory {
    Docker,
    Cache,
    Logs,
    VMs,
    Downloads,
}

impl std::fmt::Display for CleanCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CleanCategory::Docker => write!(f, "Docker"),
            CleanCategory::Cache => write!(f, "Cache"),
            CleanCategory::Logs => write!(f, "Logs"),
            CleanCategory::VMs => write!(f, "VMs"),
            CleanCategory::Downloads => write!(f, "Downloads"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum CleanCommand {
    Shell(String, Vec<String>),
    RemoveDir(PathBuf),
    RemoveFile(PathBuf),
}

// --- Path validation ---

/// Validate a path is safe to delete
pub(crate) fn is_safe_path(path: &Path) -> bool {
    let path_str = path.display().to_string();

    // Empty path
    if path_str.is_empty() {
        return false;
    }

    // Must be absolute
    if !path.is_absolute() {
        return false;
    }

    // No path traversal
    if path_str.contains("/../") || path_str.ends_with("/..") {
        return false;
    }

    // Check against protected paths
    for protected in PROTECTED_PATHS {
        if path_str == *protected {
            return false;
        }
    }

    // Don't follow symlinks to system dirs
    if path.is_symlink() {
        if let Ok(target) = std::fs::read_link(path) {
            let target_str = target.display().to_string();
            for protected in PROTECTED_PATHS {
                if target_str.starts_with(protected) && *protected != "/" {
                    return false;
                }
            }
        }
    }

    true
}

/// Check if a process is running by name
pub(crate) fn is_app_running(name: &str) -> bool {
    Command::new("pgrep")
        .args(["-xi", name])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Load user whitelist from ~/.config/spark/whitelist.txt
fn load_whitelist() -> Vec<PathBuf> {
    let config_dir = dirs::config_dir().unwrap_or_default().join("spark");
    let whitelist_path = config_dir.join("whitelist.txt");
    if !whitelist_path.exists() {
        return Vec::new();
    }
    std::fs::read_to_string(whitelist_path)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let expanded = if l.starts_with('~') {
                let home = dirs::home_dir().unwrap_or_default();
                home.join(l.strip_prefix("~/").unwrap_or(l))
            } else {
                PathBuf::from(l)
            };
            if expanded.is_absolute() {
                Some(expanded)
            } else {
                None
            }
        })
        .collect()
}

/// Check if a path is whitelisted (should be skipped)
pub(crate) fn is_whitelisted(path: &Path, whitelist: &[PathBuf]) -> bool {
    whitelist.iter().any(|w| path.starts_with(w))
}

/// Log an operation to ~/.config/spark/operations.log
fn log_operation(action: &str, path: &str, size: u64, result: &str) {
    let log_dir = dirs::config_dir().unwrap_or_default().join("spark");
    let _ = std::fs::create_dir_all(&log_dir);
    let log_path = log_dir.join("operations.log");

    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)
    {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let size_str = crate::utils::fs::format_size(size);
        let _ = writeln!(
            f,
            "[{}] {} {} {} {}",
            timestamp, action, result, size_str, path
        );
    }
}

// --- Public API ---

/// Scan for all cleanable system resources
pub fn scan_system() -> Vec<CleanableItem> {
    let whitelist = load_whitelist();
    let mut items = Vec::new();

    items.extend(scan_docker(&whitelist));
    items.extend(scan_caches(&whitelist));
    items.extend(scan_logs(&whitelist));
    items.extend(scan_vms(&whitelist));
    items.extend(scan_downloads(&whitelist));

    // Sort by category first (Docker, VMs, Cache, Logs, Downloads), then by size descending
    items.sort_by(|a, b| {
        let cat_order = |c: &CleanCategory| match c {
            CleanCategory::Docker => 0,
            CleanCategory::VMs => 1,
            CleanCategory::Cache => 2,
            CleanCategory::Logs => 3,
            CleanCategory::Downloads => 4,
        };
        cat_order(&a.category)
            .cmp(&cat_order(&b.category))
            .then(b.size.cmp(&a.size))
    });
    items
}

/// Execute a clean command with safety checks
pub fn execute_clean(item: &CleanableItem, dry_run: bool) -> Result<u64, String> {
    // Safety: check if app is running
    if item.app_running {
        let msg = format!("{}: app is running, skipping", item.name);
        log_operation("CLEAN", &item.detail, 0, "SKIPPED_APP_RUNNING");
        return Err(msg);
    }

    if dry_run {
        log_operation("DRY_RUN", &item.detail, item.size, "WOULD_CLEAN");
        return Ok(item.size);
    }

    match &item.clean_cmd {
        CleanCommand::Shell(cmd, args) => {
            // Validate the command exists
            let cmd_exists = Command::new("which")
                .arg(cmd)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);

            if !cmd_exists {
                log_operation("CLEAN", &item.detail, 0, "CMD_NOT_FOUND");
                return Err(format!("{} not found", cmd));
            }

            let output = Command::new(cmd).args(args).output().map_err(|e| {
                log_operation("CLEAN", &item.detail, 0, "FAILED");
                format!("{}: {}", cmd, e)
            })?;

            if output.status.success() {
                log_operation("CLEAN", &item.detail, item.size, "REMOVED");
                Ok(item.size)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                log_operation("CLEAN", &item.detail, 0, "FAILED");
                Err(stderr.trim().to_string())
            }
        }
        CleanCommand::RemoveDir(path) => {
            if !is_safe_path(path) {
                log_operation(
                    "CLEAN",
                    &path.display().to_string(),
                    0,
                    "BLOCKED_UNSAFE_PATH",
                );
                return Err(format!("Blocked: {} is a protected path", path.display()));
            }
            if !path.exists() {
                return Ok(0);
            }
            let size = crate::utils::fs::dir_size(path);
            std::fs::remove_dir_all(path).map_err(|e| {
                log_operation("CLEAN", &path.display().to_string(), 0, "FAILED");
                format!("{}: {}", path.display(), e)
            })?;
            log_operation("CLEAN", &path.display().to_string(), size, "REMOVED");
            Ok(size)
        }
        CleanCommand::RemoveFile(path) => {
            if !is_safe_path(path) {
                log_operation(
                    "CLEAN",
                    &path.display().to_string(),
                    0,
                    "BLOCKED_UNSAFE_PATH",
                );
                return Err(format!("Blocked: {} is a protected path", path.display()));
            }
            if !path.exists() {
                return Ok(0);
            }
            let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            std::fs::remove_file(path).map_err(|e| {
                log_operation("CLEAN", &path.display().to_string(), 0, "FAILED");
                format!("{}: {}", path.display(), e)
            })?;
            log_operation("CLEAN", &path.display().to_string(), size, "REMOVED");
            Ok(size)
        }
    }
}

// Scan category implementations are in system_categories.rs
use super::system_categories;

fn scan_docker(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    system_categories::scan_docker(whitelist)
}
fn scan_caches(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    system_categories::scan_caches(whitelist)
}
fn scan_logs(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    system_categories::scan_logs(whitelist)
}
fn scan_vms(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    system_categories::scan_vms(whitelist)
}
fn scan_downloads(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    system_categories::scan_downloads(whitelist)
}
#[cfg(test)]
fn parse_size_string(s: &str) -> u64 {
    system_categories::parse_size_string(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size_string() {
        assert_eq!(parse_size_string("1.5GB"), 1_610_612_736);
        assert_eq!(parse_size_string("100MB"), 104_857_600);
        assert_eq!(parse_size_string("512KB"), 524_288);
        assert_eq!(parse_size_string("0"), 0);
        assert_eq!(parse_size_string(""), 0);
    }

    #[test]
    fn test_is_safe_path() {
        assert!(!is_safe_path(Path::new("/")));
        assert!(!is_safe_path(Path::new("/System")));
        assert!(!is_safe_path(Path::new("/usr")));
        assert!(!is_safe_path(Path::new("/bin")));
        assert!(!is_safe_path(Path::new("")));
        assert!(!is_safe_path(Path::new("relative/path")));
        assert!(!is_safe_path(Path::new("/foo/../bar")));
        assert!(is_safe_path(Path::new("/tmp/test")));
        assert!(is_safe_path(Path::new("/Users/test/.npm")));
    }

    #[test]
    fn test_scan_system_no_panic() {
        let items = scan_system();
        let _ = items;
    }

    #[test]
    fn test_clean_categories() {
        assert_eq!(format!("{}", CleanCategory::Docker), "Docker");
        assert_eq!(format!("{}", CleanCategory::Cache), "Cache");
        assert_eq!(format!("{}", CleanCategory::Logs), "Logs");
        assert_eq!(format!("{}", CleanCategory::VMs), "VMs");
        assert_eq!(format!("{}", CleanCategory::Downloads), "Downloads");
    }

    #[test]
    fn test_protected_paths_blocked() {
        let item = CleanableItem {
            category: CleanCategory::Cache,
            name: "test".into(),
            detail: "test".into(),
            size: 100,
            clean_cmd: CleanCommand::RemoveDir(PathBuf::from("/System")),
            app_running: false,
            age_days: None,
            risk: CleanRisk::Danger,
        };
        let result = execute_clean(&item, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("protected"));
    }

    #[test]
    fn test_dry_run_does_not_delete() {
        let item = CleanableItem {
            category: CleanCategory::Cache,
            name: "test".into(),
            detail: "test".into(),
            size: 1000,
            clean_cmd: CleanCommand::RemoveDir(PathBuf::from("/tmp/nonexistent_spark_test")),
            app_running: false,
            age_days: None,
            risk: CleanRisk::Safe,
        };
        let result = execute_clean(&item, true);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 1000); // Returns estimated size
    }

    #[test]
    fn test_app_running_blocks_clean() {
        let item = CleanableItem {
            category: CleanCategory::Cache,
            name: "test".into(),
            detail: "test".into(),
            size: 100,
            clean_cmd: CleanCommand::RemoveDir(PathBuf::from("/tmp/test")),
            app_running: true,
            age_days: None,
            risk: CleanRisk::Danger,
        };
        let result = execute_clean(&item, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("running"));
    }
}
