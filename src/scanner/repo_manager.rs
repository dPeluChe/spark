//! Git repository manager: centralized clone and pull/update operations.
//!
//! Inspired by ghq but with update capabilities. Manages repos under
//! a configurable root directory (default: ~/repos) with host/owner/name layout.

use std::path::{Path, PathBuf};
use std::process::Command;

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
    /// Has uncommitted local changes
    Dirty,
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
            RepoStatus::Dirty => write!(f, "Dirty"),
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
        .args(["pull", "--ff-only"])
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

/// Fetch and check status of a repository against its remote
pub fn check_repo_status(path: &Path) -> RepoStatus {
    // Fetch first
    let _ = Command::new("git")
        .args(["fetch", "--quiet"])
        .current_dir(path)
        .output();

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
            if parts.len() == 2 {
                let ahead: usize = parts[0].parse().unwrap_or(0);
                let behind: usize = parts[1].parse().unwrap_or(0);

                if dirty {
                    RepoStatus::Dirty
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
                RepoStatus::Dirty
            } else {
                RepoStatus::UpToDate
            }
        }
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).trim().to_string();
            if dirty {
                RepoStatus::Dirty
            } else if stderr.contains("no upstream") {
                RepoStatus::UpToDate // No tracking branch
            } else {
                RepoStatus::Error(stderr)
            }
        }
        Err(e) => RepoStatus::Error(e.to_string()),
    }
}

/// List all managed repositories under a root directory
pub fn list_managed_repos(root: &Path) -> Vec<ManagedRepo> {
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
                let remote_url = get_remote_url(&repo_path);
                let branch = get_current_branch(&repo_path);
                let last_commit = get_last_commit_date(&repo_path);

                repos.push(ManagedRepo {
                    path: repo_path,
                    name,
                    remote_url,
                    branch,
                    status: RepoStatus::Checking,
                    host: host.clone(),
                    owner: owner.clone(),
                    last_commit,
                });
            }
        }
    }

    repos.sort_by(|a, b| {
        a.host.cmp(&b.host)
            .then(a.owner.cmp(&b.owner))
            .then(a.name.cmp(&b.name))
    });
    repos
}

/// Parse a git URL into (host, owner, repo_name)
fn parse_git_url(url: &str) -> Result<(String, String, String), String> {
    // Handle SSH: git@github.com:owner/repo.git
    if url.starts_with("git@") {
        let rest = &url[4..];
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

fn get_remote_url(path: &Path) -> String {
    Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_default()
}

fn get_current_branch(path: &Path) -> String {
    Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".into())
}

fn get_last_commit_date(path: &Path) -> Option<String> {
    Command::new("git")
        .args(["log", "-1", "--format=%cr"])
        .current_dir(path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if s.is_empty() { None } else { Some(s) }
            } else {
                None
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ssh_url() {
        let (host, owner, name) = parse_git_url("git@github.com:user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(owner, "user");
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_https_url() {
        let (host, owner, name) =
            parse_git_url("https://github.com/user/repo.git").unwrap();
        assert_eq!(host, "github.com");
        assert_eq!(owner, "user");
        assert_eq!(name, "repo");
    }
}
