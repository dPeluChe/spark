//! Repo metadata helpers: git URL parsing and git2-based local reads.

use std::path::Path;

/// Parse a git URL into (host, owner, repo_name)
pub(super) fn parse_git_url(url: &str) -> Result<(String, String, String), String> {
    // Handle SSH: git@github.com:owner/repo.git
    if let Some(rest) = url.strip_prefix("git@") {
        let parts: Vec<&str> = rest.splitn(2, ':').collect();
        if parts.len() == 2 {
            let host = parts[0].to_string();
            let path = parts[1].trim_end_matches(".git");
            let segments: Vec<&str> = path.splitn(2, '/').collect();
            if segments.len() == 2 {
                return Ok((host, segments[0].to_string(), segments[1].to_string()));
            }
        }
    }

    // Handle HTTPS: https://github.com/owner/repo.git
    if url.starts_with("https://") || url.starts_with("http://") {
        let without_scheme = url.split("://").nth(1).unwrap_or("");
        let parts: Vec<&str> = without_scheme.splitn(4, '/').collect();
        if parts.len() >= 3 {
            let host = parts[0].to_string();
            let owner = parts[1].to_string();
            let name = parts[2].trim_end_matches(".git").to_string();
            return Ok((host, owner, name));
        }
    }

    Err(format!("Cannot parse git URL: {}", url))
}

/// Read remote URL, current branch, and last-commit age via git2 (no subprocess)
pub(super) fn repo_metadata(path: &Path) -> (String, String, Option<String>) {
    let repo = match git2::Repository::open(path) {
        Ok(r) => r,
        Err(_) => return (String::new(), "unknown".into(), None),
    };
    let remote_url = repo
        .find_remote("origin")
        .ok()
        .and_then(|r| r.url().map(String::from))
        .unwrap_or_default();
    let head = repo.head().ok();
    let branch = head
        .as_ref()
        .and_then(|h| h.shorthand().map(String::from))
        .unwrap_or_else(|| "unknown".into());
    let last_commit = head
        .and_then(|h| h.peel_to_commit().ok())
        .map(|c| relative_age(c.time().seconds()));
    (remote_url, branch, last_commit)
}

/// Format a commit timestamp as a short relative age ("5m ago", "3d ago", "8mo ago")
pub(super) fn relative_age(secs: i64) -> String {
    let elapsed = (chrono::Utc::now().timestamp() - secs).max(0);
    const HOUR: i64 = 3600;
    const DAY: i64 = 86400;
    const MONTH: i64 = DAY * 30;
    const YEAR: i64 = DAY * 365;
    if elapsed < HOUR {
        format!("{}m ago", (elapsed / 60).max(1))
    } else if elapsed < DAY {
        format!("{}h ago", elapsed / HOUR)
    } else if elapsed < MONTH {
        format!("{}d ago", elapsed / DAY)
    } else if elapsed < YEAR {
        format!("{}mo ago", elapsed / MONTH)
    } else {
        format!("{}y ago", elapsed / YEAR)
    }
}
