//! Git repository manager: centralized clone and pull/update operations.
//!
//! Inspired by ghq but with update capabilities. Manages repos under
//! a configurable root directory (default: ~/repos) with host/owner/name layout.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Status of a managed repository
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepoStatus {
    /// Up to date with remote
    UpToDate,
    /// Local commits behind remote
    Behind(usize),
    /// Local commits ahead of remote
    Ahead(usize),
    /// Both ahead and behind
    Diverged { ahead: usize, behind: usize },
    /// Has uncommitted local changes, with its ahead/behind counts
    Dirty { ahead: usize, behind: usize },
    /// Failed to check status
    Error(String),
    /// Currently checking
    Checking,
}

impl std::fmt::Display for RepoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RepoStatus::UpToDate => write!(f, "Up to date"),
            RepoStatus::Behind(n) => write!(f, "{} behind", n),
            RepoStatus::Ahead(n) => write!(f, "{} ahead", n),
            RepoStatus::Diverged { ahead, behind } => {
                write!(f, "{} ahead, {} behind", ahead, behind)
            }
            RepoStatus::Dirty { ahead, behind } => match (ahead, behind) {
                (0, 0) => write!(f, "Dirty"),
                (0, b) => write!(f, "Dirty, {} behind", b),
                (a, 0) => write!(f, "Dirty, {} ahead", a),
                (a, b) => write!(f, "Dirty, {} ahead, {} behind", a, b),
            },
            RepoStatus::Error(e) => write!(f, "Error: {}", e),
            RepoStatus::Checking => write!(f, "Checking..."),
        }
    }
}

/// A managed git repository
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ManagedRepo {
    pub path: PathBuf,
    pub name: String,
    pub remote_url: String,
    pub branch: String,
    pub status: RepoStatus,
    pub host: String,
    pub owner: String,
    pub last_commit: Option<String>,
    pub size: u64,
}

/// Clone a repository into the managed root with host/owner/name layout.
/// Returns the path where it was cloned.
pub fn clone_repo(url: &str, root: &Path) -> Result<PathBuf, String> {
    let (host, owner, name) = parse_git_url(url)?;
    let target = root.join(&host).join(&owner).join(&name);

    if target.exists() {
        return Err(format!("Already exists: {}", target.display()));
    }

    std::fs::create_dir_all(target.parent().unwrap_or(root))
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let output = Command::new("git")
        .args(["clone", url, &target.display().to_string()])
        .output()
        .map_err(|e| format!("git clone failed: {}", e))?;

    if output.status.success() {
        Ok(target)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("git clone failed: {}", stderr.trim()))
    }
}

/// Clone a repository with --depth 1 (shallow)
pub fn clone_repo_shallow(url: &str, root: &Path) -> Result<PathBuf, String> {
    let (host, owner, name) = parse_git_url(url)?;
    let target = root.join(&host).join(&owner).join(&name);

    if target.exists() {
        return Err(format!("Already exists: {}", target.display()));
    }

    std::fs::create_dir_all(target.parent().unwrap_or(root))
        .map_err(|e| format!("Failed to create directory: {}", e))?;

    let output = Command::new("git")
        .args(["clone", "--depth", "1", url, &target.display().to_string()])
        .output()
        .map_err(|e| format!("git clone failed: {}", e))?;

    if output.status.success() {
        Ok(target)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("git clone failed: {}", stderr.trim()))
    }
}

/// Pull (fast-forward) a repository
pub fn pull_repo(path: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(["pull", "--ff-only", "--prune"])
        .current_dir(path)
        .output()
        .map_err(|e| format!("git pull failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("git pull failed: {}", stderr.trim()))
    }
}

/// Merge HEAD to the already-fetched upstream (no second fetch).
/// Call after `check_repo_status` reports `Behind`.
pub fn merge_ff_only(path: &Path) -> Result<String, String> {
    let output = Command::new("git")
        .args(["merge", "--ff-only", "@{upstream}"])
        .current_dir(path)
        .output()
        .map_err(|e| format!("git merge failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if output.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("git merge failed: {}", stderr.trim()))
    }
}

/// Fetch and check status of a repository against its remote
pub fn check_repo_status(path: &Path) -> RepoStatus {
    // Fetch first; --prune drops stale remote-tracking refs that break updates
    let fetch_err = match Command::new("git")
        .args(["fetch", "--quiet", "--prune"])
        .current_dir(path)
        .output()
    {
        Ok(o) if o.status.success() => None,
        Ok(o) => Some(String::from_utf8_lossy(&o.stderr).trim().to_string()),
        Err(e) => Some(e.to_string()),
    };

    // Check for dirty working tree
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(path)
        .output()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false);

    // Get ahead/behind counts
    let output = Command::new("git")
        .args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"])
        .current_dir(path)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let text = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let parts: Vec<&str> = text.split_whitespace().collect();
            let status = if parts.len() == 2 {
                let ahead: usize = parts[0].parse().unwrap_or(0);
                let behind: usize = parts[1].parse().unwrap_or(0);

                if dirty {
                    RepoStatus::Dirty { ahead, behind }
                } else if ahead > 0 && behind > 0 {
                    RepoStatus::Diverged { ahead, behind }
                } else if behind > 0 {
                    RepoStatus::Behind(behind)
                } else if ahead > 0 {
                    RepoStatus::Ahead(ahead)
                } else {
                    RepoStatus::UpToDate
                }
            } else if dirty {
                RepoStatus::Dirty {
                    ahead: 0,
                    behind: 0,
                }
            } else {
                RepoStatus::UpToDate
            };
            match (status, fetch_err) {
                // 0/0 against upstream is the only state that proves nothing when
                // the fetch failed — report the fetch error instead of a lie.
                (RepoStatus::UpToDate, Some(e)) => {
                    RepoStatus::Error(format!("fetch failed: {}", e))
                }
                (s, _) => s,
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            if dirty {
                RepoStatus::Dirty {
                    ahead: 0,
                    behind: 0,
                }
            } else if stderr.contains("no upstream") {
                RepoStatus::UpToDate // No tracking branch
            } else {
                RepoStatus::Error(stderr)
            }
        }
        Err(e) => RepoStatus::Error(e.to_string()),
    }
}

/// Max concurrent git fetches when checking many repos.
pub const STATUS_CONCURRENCY: usize = 8;

/// Check statuses for many repos in parallel (bounded to `STATUS_CONCURRENCY`
/// threads). Prints a `\r` progress counter to stderr.
pub fn check_statuses_parallel(repos: &[&ManagedRepo]) -> Vec<RepoStatus> {
    let total = repos.len();
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let (tx, rx) = std::sync::mpsc::channel::<(usize, RepoStatus)>();

    std::thread::scope(|s| {
        let next = &next;
        let done = &done;
        for _ in 0..STATUS_CONCURRENCY.min(total) {
            let tx = tx.clone();
            s.spawn(move || loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= total {
                    break;
                }
                let status = check_repo_status(&repos[i].path);
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                eprint!("\r  [{}/{}] {}/{}", n, total, repos[i].owner, repos[i].name);
                let _ = tx.send((i, status));
            });
        }
    });
    drop(tx);
    eprintln!("\r{}\r", " ".repeat(60));

    let mut statuses = vec![RepoStatus::Checking; total];
    for (i, status) in rx {
        statuses[i] = status;
    }
    statuses
}

/// List all managed repositories under a root directory, including each
/// repo's `.git` size (walks the object store — use only when displayed).
pub fn list_managed_repos(root: &Path) -> Vec<ManagedRepo> {
    list_repos(root, true)
}

/// Same as `list_managed_repos` but skips the `.git` size walk (`size = 0`).
/// For callers that never display it (cd/search/list/status/pull).
pub fn list_managed_repos_lite(root: &Path) -> Vec<ManagedRepo> {
    list_repos(root, false)
}

fn list_repos(root: &Path, with_size: bool) -> Vec<ManagedRepo> {
    let mut repos = Vec::new();

    if !root.exists() {
        return repos;
    }

    // Walk 3 levels: root/host/owner/repo
    let hosts = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return repos,
    };

    for host_entry in hosts.filter_map(|e| e.ok()) {
        if !host_entry.path().is_dir() {
            continue;
        }
        let host = host_entry.file_name().to_string_lossy().to_string();

        let owners = match std::fs::read_dir(host_entry.path()) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for owner_entry in owners.filter_map(|e| e.ok()) {
            if !owner_entry.path().is_dir() {
                continue;
            }
            let owner = owner_entry.file_name().to_string_lossy().to_string();

            let repo_entries = match std::fs::read_dir(owner_entry.path()) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for repo_entry in repo_entries.filter_map(|e| e.ok()) {
                let repo_path = repo_entry.path();
                if !repo_path.join(".git").exists() {
                    continue;
                }

                let name = repo_entry.file_name().to_string_lossy().to_string();
                let (remote_url, branch, last_commit) = repo_metadata(&repo_path);
                let size = if with_size {
                    crate::utils::fs::dir_size(&repo_path.join(".git"))
                } else {
                    0
                };

                repos.push(ManagedRepo {
                    path: repo_path,
                    name,
                    remote_url,
                    branch,
                    status: RepoStatus::Checking,
                    host: host.clone(),
                    owner: owner.clone(),
                    last_commit,
                    size,
                });
            }
        }
    }

    repos.sort_by(|a, b| {
        a.host
            .cmp(&b.host)
            .then(a.owner.cmp(&b.owner))
            .then(a.name.cmp(&b.name))
    });
    repos
}

mod cache;
mod meta;
pub use cache::*;
use meta::{parse_git_url, repo_metadata};

#[cfg(test)]
mod tests;
