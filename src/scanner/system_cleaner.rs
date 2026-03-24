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

const LOG_AGE_DAYS: u64 = 7;

/// Protected paths that must NEVER be deleted
const PROTECTED_PATHS: &[&str] = &[
    "/", "/System", "/bin", "/sbin", "/usr", "/etc",
    "/var/db", "/Library/Extensions", "/private/var/db",
    "/Applications", "/Library",
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
fn is_safe_path(path: &Path) -> bool {
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
fn is_app_running(name: &str) -> bool {
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
            if expanded.is_absolute() { Some(expanded) } else { None }
        })
        .collect()
}

/// Check if a path is whitelisted (should be skipped)
fn is_whitelisted(path: &Path, whitelist: &[PathBuf]) -> bool {
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
        let _ = writeln!(f, "[{}] {} {} {} {}", timestamp, action, result, size_str, path);
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
        cat_order(&a.category).cmp(&cat_order(&b.category))
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

            let output = Command::new(cmd)
                .args(args)
                .output()
                .map_err(|e| {
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
                log_operation("CLEAN", &path.display().to_string(), 0, "BLOCKED_UNSAFE_PATH");
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
                log_operation("CLEAN", &path.display().to_string(), 0, "BLOCKED_UNSAFE_PATH");
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

// --- Docker scanning ---

fn scan_docker(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    let mut items = Vec::new();

    let docker_ok = Command::new("docker")
        .args(["info", "--format", "{{.ID}}"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !docker_ok {
        return items;
    }

    // Dangling images
    if let Some(mut item) = scan_docker_category(
        &["images", "-f", "dangling=true", "--format", "{{.Size}}"],
        "Dangling images",
        vec!["image", "prune", "-f"],
    ) {
        if !is_whitelisted(&PathBuf::from("/var/lib/docker"), whitelist) {
            item.app_running = false; // Docker is running (we verified above)
            items.push(item);
        }
    }

    // Stopped containers
    if let Some(mut item) = scan_docker_category(
        &["ps", "-a", "-f", "status=exited", "--format", "{{.Size}}"],
        "Stopped containers",
        vec!["container", "prune", "-f"],
    ) {
        item.app_running = false;
        items.push(item);
    }

    // Build cache
    if let Some(item) = scan_docker_build_cache() {
        items.push(item);
    }

    items
}

fn scan_docker_category(query_args: &[&str], name: &str, clean_args: Vec<&str>) -> Option<CleanableItem> {
    let output = Command::new("docker").args(query_args).output().ok()?;
    if !output.status.success() { return None; }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.lines().filter(|l| !l.is_empty()).count();
    if count == 0 { return None; }

    let total_size = parse_docker_sizes(&stdout);

    Some(CleanableItem {
        category: CleanCategory::Docker,
        name: name.into(),
        detail: format!("{} items", count),
        size: total_size,
        clean_cmd: CleanCommand::Shell(
            "docker".into(),
            clean_args.iter().map(|s| s.to_string()).collect(),
        ),
        app_running: false,
        age_days: None,
    })
}

fn scan_docker_build_cache() -> Option<CleanableItem> {
    let output = Command::new("docker")
        .args(["system", "df", "--format", "{{.Type}}\t{{.Size}}\t{{.Reclaimable}}"])
        .output().ok()?;

    if !output.status.success() { return None; }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 && parts[0] == "Build Cache" {
            let reclaimable = parse_size_string(parts[2].split_whitespace().next().unwrap_or("0"));
            if reclaimable > 0 {
                return Some(CleanableItem {
                    category: CleanCategory::Docker,
                    name: "Build cache".into(),
                    detail: format!("{} reclaimable", crate::utils::fs::format_size(reclaimable)),
                    size: reclaimable,
                    clean_cmd: CleanCommand::Shell(
                        "docker".into(),
                        vec!["builder".into(), "prune".into(), "-f".into()],
                    ),
                    app_running: false,
                    age_days: None,
                });
            }
        }
    }
    None
}

// --- Cache scanning ---

fn scan_caches(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    let mut items = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();

    struct CacheTarget {
        name: &'static str,
        path: PathBuf,
        cmd: &'static str,
        args: Vec<&'static str>,
        app_process: &'static str,
    }

    let targets = vec![
        CacheTarget {
            name: "Homebrew cache",
            path: home.join("Library/Caches/Homebrew"),
            cmd: "brew", args: vec!["cleanup", "--prune=all"],
            app_process: "",
        },
        CacheTarget {
            name: "npm cache",
            path: home.join(".npm/_cacache"),
            cmd: "npm", args: vec!["cache", "clean", "--force"],
            app_process: "",
        },
        CacheTarget {
            name: "pip cache",
            path: home.join("Library/Caches/pip"),
            cmd: "pip3", args: vec!["cache", "purge"],
            app_process: "",
        },
        CacheTarget {
            name: "Cargo registry",
            path: home.join(".cargo/registry"),
            cmd: "", args: vec![],
            app_process: "",
        },
        CacheTarget {
            name: "Xcode DerivedData",
            path: home.join("Library/Developer/Xcode/DerivedData"),
            cmd: "", args: vec![],
            app_process: "Xcode",
        },
        CacheTarget {
            name: "CocoaPods cache",
            path: home.join("Library/Caches/CocoaPods"),
            cmd: "pod", args: vec!["cache", "clean", "--all"],
            app_process: "",
        },
        CacheTarget {
            name: "Go module cache",
            path: home.join("go/pkg/mod/cache"),
            cmd: "go", args: vec!["clean", "-modcache"],
            app_process: "",
        },
        CacheTarget {
            name: "Gradle cache",
            path: home.join(".gradle/caches"),
            cmd: "", args: vec![],
            app_process: "java",
        },
    ];

    for target in &targets {
        if !target.path.exists() {
            continue;
        }
        if is_whitelisted(&target.path, whitelist) {
            continue;
        }

        let size = crate::utils::fs::dir_size(&target.path);
        if size < 1_048_576 { // < 1MB skip
            continue;
        }

        let app_running = if target.app_process.is_empty() {
            false
        } else {
            is_app_running(target.app_process)
        };

        let clean_cmd = if !target.cmd.is_empty() && !target.args.is_empty() {
            let cmd_exists = Command::new("which")
                .arg(target.cmd)
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if cmd_exists {
                CleanCommand::Shell(
                    target.cmd.to_string(),
                    target.args.iter().map(|s| s.to_string()).collect(),
                )
            } else {
                CleanCommand::RemoveDir(target.path.clone())
            }
        } else {
            CleanCommand::RemoveDir(target.path.clone())
        };

        let home_str = home.display().to_string();
        let detail = {
            let p = target.path.display().to_string();
            if p.starts_with(&home_str) { format!("~{}", &p[home_str.len()..]) } else { p }
        };

        let mut warn = String::new();
        if app_running {
            warn = format!(" ({}  running)", target.app_process);
        }

        items.push(CleanableItem {
            category: CleanCategory::Cache,
            name: target.name.to_string(),
            detail: format!("{}{}", detail, warn),
            size,
            clean_cmd,
            app_running,
            age_days: None,
        });
    }

    items
}

// --- Log scanning ---

fn scan_logs(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    let mut items = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();

    let log_dirs = [
        home.join(".context"),
        home.join(".codex"),
        home.join(".factory"),
        home.join("Library/Logs"),
    ];

    let now = std::time::SystemTime::now();

    for dir in &log_dirs {
        if !dir.exists() { continue; }

        for entry in walkdir::WalkDir::new(dir)
            .max_depth(3)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() { continue; }
            let path = entry.path();

            if is_whitelisted(path, whitelist) { continue; }

            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "log" && ext != "hprof" { continue; }

            let meta = match std::fs::metadata(path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let size = meta.len();
            if size < 10_485_760 { continue; } // < 10MB skip

            // Age-based: only include logs older than LOG_AGE_DAYS
            let age_days = meta.modified().ok()
                .and_then(|m| now.duration_since(m).ok())
                .map(|d| d.as_secs() / 86400);

            if let Some(days) = age_days {
                if days < LOG_AGE_DAYS {
                    continue; // Too recent, skip
                }
            }

            let name = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            let home_str = home.display().to_string();
            let display = {
                let p = path.display().to_string();
                if p.starts_with(&home_str) { format!("~{}", &p[home_str.len()..]) } else { p }
            };

            items.push(CleanableItem {
                category: CleanCategory::Logs,
                name,
                detail: display,
                size,
                clean_cmd: CleanCommand::RemoveFile(path.to_path_buf()),
                app_running: false,
                age_days,
            });
        }
    }

    items
}

// --- VM scanning ---

fn scan_vms(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    let mut items = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();

    // Docker VM disk
    let docker_raw = home.join("Library/Containers/com.docker.docker/Data/vms/0/Docker.raw");
    if docker_raw.exists() && !is_whitelisted(&docker_raw, whitelist) {
        let size = std::fs::metadata(&docker_raw).map(|m| m.len()).unwrap_or(0);
        if size > 100_000_000 {
            items.push(CleanableItem {
                category: CleanCategory::VMs,
                name: "Docker VM disk".into(),
                detail: "Reset via Docker Desktop > Troubleshoot > Clean/Purge data".into(),
                size,
                // Don't auto-delete Docker.raw — guide user to Docker Desktop
                clean_cmd: CleanCommand::Shell(
                    "docker".into(),
                    vec!["system".into(), "prune".into(), "-a".into(), "-f".into()],
                ),
                app_running: is_app_running("Docker"),
                age_days: None,
            });
        }
    }

    // Android emulator AVDs
    let avd_dir = home.join(".android/avd");
    if avd_dir.exists() && !is_whitelisted(&avd_dir, whitelist) {
        let size = crate::utils::fs::dir_size(&avd_dir);
        if size > 100_000_000 {
            items.push(CleanableItem {
                category: CleanCategory::VMs,
                name: "Android emulators".into(),
                detail: "~/.android/avd".into(),
                size,
                clean_cmd: CleanCommand::RemoveDir(avd_dir),
                app_running: is_app_running("qemu-system"),
                age_days: None,
            });
        }
    }

    // Old IE VMs
    let ievms = home.join(".ievms");
    if ievms.exists() && !is_whitelisted(&ievms, whitelist) {
        let size = crate::utils::fs::dir_size(&ievms);
        if size > 1_000_000 {
            items.push(CleanableItem {
                category: CleanCategory::VMs,
                name: "IE test VMs".into(),
                detail: "~/.ievms (legacy IE testing)".into(),
                size,
                clean_cmd: CleanCommand::RemoveDir(ievms),
                app_running: false,
                age_days: None,
            });
        }
    }

    // boot2docker legacy
    let b2d = home.join(".docker/machine");
    if b2d.exists() && !is_whitelisted(&b2d, whitelist) {
        let size = crate::utils::fs::dir_size(&b2d);
        if size > 1_000_000 {
            items.push(CleanableItem {
                category: CleanCategory::VMs,
                name: "boot2docker".into(),
                detail: "~/.docker/machine (legacy)".into(),
                size,
                clean_cmd: CleanCommand::RemoveDir(b2d),
                app_running: false,
                age_days: None,
            });
        }
    }

    items
}

// --- Download scanning ---

fn scan_downloads(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    let mut items = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();
    let downloads = home.join("Downloads");

    if !downloads.exists() { return items; }

    let entries = match std::fs::read_dir(&downloads) {
        Ok(e) => e,
        Err(_) => return items,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() { continue; }
        if is_whitelisted(&path, whitelist) { continue; }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if !matches!(ext.as_str(), "iso" | "dmg" | "pkg" | "ova" | "ovf" | "vmdk") {
            continue;
        }
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size < 50_000_000 { continue; } // < 50MB skip

        let name = path.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        items.push(CleanableItem {
            category: CleanCategory::Downloads,
            name: name.clone(),
            detail: format!("~/Downloads/{}", name),
            size,
            clean_cmd: CleanCommand::RemoveFile(path),
            app_running: false,
            age_days: None,
        });
    }

    items
}

// --- Helpers ---

fn parse_docker_sizes(output: &str) -> u64 {
    output.lines()
        .map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            parts.last().map(|s| parse_size_string(s)).unwrap_or(0)
        })
        .sum()
}

fn parse_size_string(s: &str) -> u64 {
    let s = s.trim();
    if s.is_empty() { return 0; }

    let mut num_end = 0;
    for (i, c) in s.char_indices() {
        if c.is_ascii_digit() || c == '.' {
            num_end = i + c.len_utf8();
        } else {
            break;
        }
    }

    let number: f64 = s[..num_end].parse().unwrap_or(0.0);
    let unit = s[num_end..].trim().to_lowercase();

    match unit.as_str() {
        "b" | "bytes" => number as u64,
        "kb" | "k" => (number * 1024.0) as u64,
        "mb" | "m" => (number * 1_048_576.0) as u64,
        "gb" | "g" => (number * 1_073_741_824.0) as u64,
        "tb" | "t" => (number * 1_099_511_627_776.0) as u64,
        _ => (number * 1_048_576.0) as u64,
    }
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
        };
        let result = execute_clean(&item, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("running"));
    }
}
