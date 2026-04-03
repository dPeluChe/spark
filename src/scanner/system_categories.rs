//! System cleanup scan categories: Docker, caches, logs, VMs, downloads.
//! Extracted from system_cleaner.rs for maintainability.

use std::path::PathBuf;
use std::process::Command;
use super::system_cleaner::*;

// --- Docker ---

pub fn scan_docker(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    let mut items = Vec::new();
    let docker_ok = Command::new("docker")
        .args(["info", "--format", "{{.ID}}"])
        .output().map(|o| o.status.success()).unwrap_or(false);
    if !docker_ok { return items; }

    if let Some(mut item) = scan_docker_category(
        &["images", "-f", "dangling=true", "--format", "{{.Size}}"],
        "Dangling images", vec!["image", "prune", "-f"],
    ) {
        if !is_whitelisted(&PathBuf::from("/var/lib/docker"), whitelist) {
            item.app_running = false;
            items.push(item);
        }
    }
    if let Some(mut item) = scan_docker_category(
        &["ps", "-a", "-f", "status=exited", "--format", "{{.Size}}"],
        "Stopped containers", vec!["container", "prune", "-f"],
    ) {
        item.app_running = false;
        items.push(item);
    }
    if let Some(item) = scan_docker_build_cache() { items.push(item); }
    items
}

fn scan_docker_category(query_args: &[&str], name: &str, clean_args: Vec<&str>) -> Option<CleanableItem> {
    let output = Command::new("docker").args(query_args).output().ok()?;
    if !output.status.success() { return None; }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let count = stdout.lines().filter(|l| !l.is_empty()).count();
    if count == 0 { return None; }
    Some(CleanableItem {
        category: CleanCategory::Docker, name: name.into(),
        detail: format!("{} items", count), size: parse_docker_sizes(&stdout),
        clean_cmd: CleanCommand::Shell("docker".into(), clean_args.iter().map(|s| s.to_string()).collect()),
        app_running: false, age_days: None, risk: CleanRisk::Caution,
    })
}

fn scan_docker_build_cache() -> Option<CleanableItem> {
    let output = Command::new("docker")
        .args(["system", "df", "--format", "{{.Type}}\t{{.Size}}\t{{.Reclaimable}}"])
        .output().ok()?;
    if !output.status.success() { return None; }
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 && parts[0] == "Build Cache" {
            let reclaimable = parse_size_string(parts[2].split_whitespace().next().unwrap_or("0"));
            if reclaimable > 0 {
                return Some(CleanableItem {
                    category: CleanCategory::Docker, name: "Build cache".into(),
                    detail: format!("{} reclaimable", crate::utils::fs::format_size(reclaimable)),
                    size: reclaimable,
                    clean_cmd: CleanCommand::Shell("docker".into(), vec!["builder".into(), "prune".into(), "-f".into()]),
                    app_running: false, age_days: None, risk: CleanRisk::Caution,
                });
            }
        }
    }
    None
}

// --- Caches ---

pub fn scan_caches(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    let mut items = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();

    struct Target { name: &'static str, path: PathBuf, cmd: &'static str, args: Vec<&'static str>, app: &'static str }

    let targets = vec![
        Target { name: "Homebrew cache", path: home.join("Library/Caches/Homebrew"), cmd: "brew", args: vec!["cleanup", "--prune=all"], app: "" },
        Target { name: "npm cache", path: home.join(".npm/_cacache"), cmd: "npm", args: vec!["cache", "clean", "--force"], app: "" },
        Target { name: "pip cache", path: home.join("Library/Caches/pip"), cmd: "pip3", args: vec!["cache", "purge"], app: "" },
        Target { name: "Cargo registry", path: home.join(".cargo/registry"), cmd: "", args: vec![], app: "" },
        Target { name: "Xcode DerivedData", path: home.join("Library/Developer/Xcode/DerivedData"), cmd: "", args: vec![], app: "Xcode" },
        Target { name: "CocoaPods cache", path: home.join("Library/Caches/CocoaPods"), cmd: "pod", args: vec!["cache", "clean", "--all"], app: "" },
        Target { name: "Go module cache", path: home.join("go/pkg/mod/cache"), cmd: "go", args: vec!["clean", "-modcache"], app: "" },
        Target { name: "Gradle cache", path: home.join(".gradle/caches"), cmd: "", args: vec![], app: "java" },
    ];

    for t in &targets {
        if !t.path.exists() || is_whitelisted(&t.path, whitelist) { continue; }
        let size = crate::utils::fs::dir_size(&t.path);
        if size < 1_048_576 { continue; }
        let app_running = if t.app.is_empty() { false } else { is_app_running(t.app) };
        let clean_cmd = if !t.cmd.is_empty() && !t.args.is_empty() {
            let exists = Command::new("which").arg(t.cmd).output().map(|o| o.status.success()).unwrap_or(false);
            if exists { CleanCommand::Shell(t.cmd.to_string(), t.args.iter().map(|s| s.to_string()).collect()) }
            else { CleanCommand::RemoveDir(t.path.clone()) }
        } else { CleanCommand::RemoveDir(t.path.clone()) };
        let home_str = home.display().to_string();
        let detail = { let p = t.path.display().to_string();
            if p.starts_with(&home_str) { format!("~{}", &p[home_str.len()..]) } else { p } };
        let warn = if app_running { format!(" ({} running)", t.app) } else { String::new() };
        items.push(CleanableItem {
            category: CleanCategory::Cache, name: t.name.to_string(),
            detail: format!("{}{}", detail, warn), size, clean_cmd, app_running, age_days: None,
            risk: if app_running { CleanRisk::Danger } else { CleanRisk::Safe },
        });
    }
    items
}

// --- Logs ---

pub fn scan_logs(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    let mut items = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();
    let log_dirs = [home.join(".context"), home.join(".codex"), home.join(".factory"), home.join("Library/Logs")];
    let now = std::time::SystemTime::now();

    for dir in &log_dirs {
        if !dir.exists() { continue; }
        for entry in walkdir::WalkDir::new(dir).max_depth(3).into_iter().filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() { continue; }
            let path = entry.path();
            if is_whitelisted(path, whitelist) { continue; }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "log" && ext != "hprof" { continue; }
            let meta = match std::fs::metadata(path) { Ok(m) => m, Err(_) => continue };
            let size = meta.len();
            if size < 10_485_760 { continue; }
            let age_days = meta.modified().ok().and_then(|m| now.duration_since(m).ok()).map(|d| d.as_secs() / 86400);
            if let Some(days) = age_days { if days < super::system_cleaner::LOG_AGE_DAYS { continue; } }
            let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            let home_str = home.display().to_string();
            let display = { let p = path.display().to_string();
                if p.starts_with(&home_str) { format!("~{}", &p[home_str.len()..]) } else { p } };
            items.push(CleanableItem {
                category: CleanCategory::Logs, name, detail: display, size,
                clean_cmd: CleanCommand::RemoveFile(path.to_path_buf()), app_running: false, age_days,
                risk: CleanRisk::Safe,
            });
        }
    }
    items
}

// --- VMs ---

pub fn scan_vms(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    let mut items = Vec::new();
    let home = dirs::home_dir().unwrap_or_default();

    let docker_raw = home.join("Library/Containers/com.docker.docker/Data/vms/0/Docker.raw");
    if docker_raw.exists() && !is_whitelisted(&docker_raw, whitelist) {
        let size = std::fs::metadata(&docker_raw).map(|m| m.len()).unwrap_or(0);
        if size > 100_000_000 {
            items.push(CleanableItem {
                category: CleanCategory::VMs, name: "Docker VM disk".into(),
                detail: "Reset via Docker Desktop > Troubleshoot > Clean/Purge data".into(), size,
                clean_cmd: CleanCommand::Shell("docker".into(), vec!["system".into(), "prune".into(), "-a".into(), "-f".into()]),
                app_running: is_app_running("Docker"), age_days: None,
                risk: CleanRisk::Caution,
            });
        }
    }
    let avd_dir = home.join(".android/avd");
    if avd_dir.exists() && !is_whitelisted(&avd_dir, whitelist) {
        let size = crate::utils::fs::dir_size(&avd_dir);
        if size > 100_000_000 {
            items.push(CleanableItem {
                category: CleanCategory::VMs, name: "Android emulators".into(), detail: "~/.android/avd".into(),
                size, clean_cmd: CleanCommand::RemoveDir(avd_dir), app_running: is_app_running("qemu-system"), age_days: None,
                risk: CleanRisk::Caution,
            });
        }
    }
    let ievms = home.join(".ievms");
    if ievms.exists() && !is_whitelisted(&ievms, whitelist) {
        let size = crate::utils::fs::dir_size(&ievms);
        if size > 1_000_000 {
            items.push(CleanableItem {
                category: CleanCategory::VMs, name: "IE test VMs".into(), detail: "~/.ievms (legacy IE testing)".into(),
                size, clean_cmd: CleanCommand::RemoveDir(ievms), app_running: false, age_days: None,
                risk: CleanRisk::Safe,
            });
        }
    }
    let b2d = home.join(".docker/machine");
    if b2d.exists() && !is_whitelisted(&b2d, whitelist) {
        let size = crate::utils::fs::dir_size(&b2d);
        if size > 1_000_000 {
            items.push(CleanableItem {
                category: CleanCategory::VMs, name: "boot2docker".into(), detail: "~/.docker/machine (legacy)".into(),
                size, clean_cmd: CleanCommand::RemoveDir(b2d), app_running: false, age_days: None,
                risk: CleanRisk::Safe,
            });
        }
    }
    items
}

// --- Downloads ---

pub fn scan_downloads(whitelist: &[PathBuf]) -> Vec<CleanableItem> {
    let mut items = Vec::new();
    let downloads = dirs::home_dir().unwrap_or_default().join("Downloads");
    if !downloads.exists() { return items; }
    let entries = match std::fs::read_dir(&downloads) { Ok(e) => e, Err(_) => return items };
    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() || is_whitelisted(&path, whitelist) { continue; }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if !matches!(ext.as_str(), "iso" | "dmg" | "pkg" | "ova" | "ovf" | "vmdk") { continue; }
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        if size < 50_000_000 { continue; }
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        items.push(CleanableItem {
            category: CleanCategory::Downloads, name: name.clone(), detail: format!("~/Downloads/{}", name),
            size, clean_cmd: CleanCommand::RemoveFile(path), app_running: false, age_days: None,
            risk: CleanRisk::Safe,
        });
    }
    items
}

// --- Helpers ---

pub fn parse_docker_sizes(output: &str) -> u64 {
    output.lines().map(|line| {
        let parts: Vec<&str> = line.split_whitespace().collect();
        parts.last().map(|s| parse_size_string(s)).unwrap_or(0)
    }).sum()
}

pub fn parse_size_string(s: &str) -> u64 {
    let s = s.trim();
    if s.is_empty() { return 0; }
    let mut num_end = 0;
    for (i, c) in s.char_indices() {
        if c.is_ascii_digit() || c == '.' { num_end = i + c.len_utf8(); } else { break; }
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
