//! Status cache: per-repo ahead/behind state persisted as JSON with a 4h TTL.

use super::RepoStatus;
use std::path::{Path, PathBuf};

const CACHE_EXPIRY_HOURS: u64 = 4;

/// Cache file path
fn cache_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("spark")
        .join("repo_status_cache.json")
}

/// Clear the status cache (forces fresh fetch on next check)
pub fn clear_status_cache() {
    let path = cache_path();
    let _ = std::fs::remove_file(path);
}

/// Load cached statuses. Returns map of repo_path -> (status_string, timestamp)
pub fn load_status_cache() -> std::collections::HashMap<String, (String, u64)> {
    let path = cache_path();
    if !path.exists() {
        return std::collections::HashMap::new();
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return std::collections::HashMap::new(),
    };
    serde_json::from_str(&content).unwrap_or_default()
}

/// Save a single status entry to cache
pub fn save_status_to_cache(repo_path: &str, status: &str) {
    save_statuses_to_cache([(repo_path.to_string(), status.to_string())]);
}

/// Save many status entries with a single read+write of the cache file
pub fn save_statuses_to_cache<I>(updates: I)
where
    I: IntoIterator<Item = (String, String)>,
{
    let mut cache = load_status_cache();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    for (path, status) in updates {
        cache.insert(path, (status, now));
    }

    let cache_file = cache_path();
    let _ = std::fs::create_dir_all(cache_file.parent().unwrap_or(Path::new("/tmp")));
    let _ = std::fs::write(
        cache_file,
        serde_json::to_string(&cache).unwrap_or_default(),
    );
}

/// Check if a cached entry is still valid
pub fn is_cache_valid(timestamp: u64) -> bool {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    now.saturating_sub(timestamp) < CACHE_EXPIRY_HOURS * 3600
}

/// Convert RepoStatus to a cacheable string
pub fn status_to_string(status: &RepoStatus) -> String {
    match status {
        RepoStatus::UpToDate => "up_to_date".into(),
        RepoStatus::Behind(n) => format!("behind:{}", n),
        RepoStatus::Ahead(n) => format!("ahead:{}", n),
        RepoStatus::Diverged { ahead, behind } => format!("diverged:{}:{}", ahead, behind),
        RepoStatus::Dirty => "dirty".into(),
        RepoStatus::Error(e) => format!("error:{}", e),
        RepoStatus::Checking => "checking".into(),
    }
}

/// Convert cached string back to RepoStatus
pub fn string_to_status(s: &str) -> RepoStatus {
    if s == "up_to_date" {
        return RepoStatus::UpToDate;
    }
    if s == "dirty" {
        return RepoStatus::Dirty;
    }
    if s == "checking" {
        return RepoStatus::Checking;
    }
    if let Some(n) = s.strip_prefix("behind:") {
        return RepoStatus::Behind(n.parse().unwrap_or(0));
    }
    if let Some(n) = s.strip_prefix("ahead:") {
        return RepoStatus::Ahead(n.parse().unwrap_or(0));
    }
    if let Some(rest) = s.strip_prefix("diverged:") {
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() == 2 {
            return RepoStatus::Diverged {
                ahead: parts[0].parse().unwrap_or(0),
                behind: parts[1].parse().unwrap_or(0),
            };
        }
    }
    if let Some(e) = s.strip_prefix("error:") {
        return RepoStatus::Error(e.to_string());
    }
    RepoStatus::Checking
}
