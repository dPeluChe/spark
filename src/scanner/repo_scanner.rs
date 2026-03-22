use std::path::PathBuf;
use chrono::{DateTime, Utc};
use crate::scanner::space_analyzer::ArtifactInfo;

/// Status of git in a repository
#[derive(Debug, Clone)]
pub enum RepoGitStatus {
    Clean,
    Dirty {
        untracked: usize,
        modified: usize,
        staged: usize,
    },
    NotARepo,
}

/// Full information about a discovered repository
#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub path: PathBuf,
    pub name: String,
    pub last_commit_date: Option<DateTime<Utc>>,
    pub last_modified: Option<std::time::SystemTime>,
    pub total_size: u64,
    pub artifact_size: u64,
    pub git_status: RepoGitStatus,
    pub branch: String,
    pub remote_url: Option<String>,
    pub has_remote: bool,
    pub commit_count: usize,
    pub is_dirty: bool,
    pub artifacts: Vec<ArtifactInfo>,
    pub health_score: u8,
    pub health_grade: HealthGrade,
}

/// Health grade for a repository
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealthGrade {
    A,
    B,
    C,
    D,
    F,
}

impl std::fmt::Display for HealthGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthGrade::A => write!(f, "A"),
            HealthGrade::B => write!(f, "B"),
            HealthGrade::C => write!(f, "C"),
            HealthGrade::D => write!(f, "D"),
            HealthGrade::F => write!(f, "F"),
        }
    }
}

/// Scan directories for git repositories
pub async fn scan_directories(
    dirs: &[PathBuf],
    max_depth: usize,
    tx: tokio::sync::mpsc::UnboundedSender<ScanProgressMsg>,
) -> Vec<RepoInfo> {
    let mut repos = Vec::new();
    let mut dirs_scanned = 0usize;

    for dir in dirs {
        if !dir.exists() {
            continue;
        }

        for entry in walkdir::WalkDir::new(dir)
            .max_depth(max_depth)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            dirs_scanned += 1;

            if entry.file_type().is_dir() && entry.file_name() == ".git" {
                if let Some(parent) = entry.path().parent() {
                    if let Some(repo) = analyze_repo(parent) {
                        repos.push(repo);
                        let _ = tx.send(ScanProgressMsg {
                            repos_found: repos.len(),
                            dirs_scanned,
                            current_dir: parent.display().to_string(),
                        });
                    }
                }
            }
        }
    }

    repos
}

/// Scan progress message
#[derive(Debug, Clone)]
pub struct ScanProgressMsg {
    pub repos_found: usize,
    pub dirs_scanned: usize,
    pub current_dir: String,
}

/// Analyze a single repository using git2
pub fn analyze_repo(path: &std::path::Path) -> Option<RepoInfo> {
    use crate::scanner::health::calculate_health;
    use crate::scanner::space_analyzer::find_artifacts;

    let repo = git2::Repository::open(path).ok()?;

    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into());

    // Get last commit date
    let last_commit_date = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .map(|c| {
            let time = c.time();
            DateTime::from_timestamp(time.seconds(), 0).unwrap_or_default()
        });

    // Get branch name
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(String::from))
        .unwrap_or_else(|| "detached".into());

    // Check remotes
    let remotes = repo.remotes().ok();
    let has_remote = remotes.as_ref().map(|r| !r.is_empty()).unwrap_or(false);
    let remote_url = if has_remote {
        repo.find_remote("origin")
            .ok()
            .and_then(|r| r.url().map(String::from))
    } else {
        None
    };

    // Check dirty status
    let statuses = repo
        .statuses(Some(
            git2::StatusOptions::new()
                .include_untracked(true)
                .recurse_untracked_dirs(false),
        ))
        .ok();

    let (git_status, is_dirty) = match &statuses {
        Some(s) => {
            let mut untracked = 0;
            let mut modified = 0;
            let mut staged = 0;
            for entry in s.iter() {
                let status = entry.status();
                if status.contains(git2::Status::WT_NEW) {
                    untracked += 1;
                }
                if status.contains(git2::Status::WT_MODIFIED)
                    || status.contains(git2::Status::WT_DELETED)
                {
                    modified += 1;
                }
                if status.contains(git2::Status::INDEX_NEW)
                    || status.contains(git2::Status::INDEX_MODIFIED)
                    || status.contains(git2::Status::INDEX_DELETED)
                {
                    staged += 1;
                }
            }
            let dirty = untracked > 0 || modified > 0 || staged > 0;
            (
                RepoGitStatus::Dirty {
                    untracked,
                    modified,
                    staged,
                },
                dirty,
            )
        }
        None => (RepoGitStatus::Clean, false),
    };

    // Count commits (cap at 1000)
    let commit_count = count_commits(&repo, 1000);

    // Artifacts
    let artifacts = find_artifacts(path);
    let artifact_size: u64 = artifacts.iter().map(|a| a.size).sum();

    // Total size
    let total_size = crate::utils::fs::dir_size(path);

    // Last modified
    let last_modified = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());

    // Health score
    let (health_score, health_grade) =
        calculate_health(last_commit_date, last_modified, has_remote, is_dirty, artifact_size);

    Some(RepoInfo {
        path: path.to_path_buf(),
        name,
        last_commit_date,
        last_modified,
        total_size,
        artifact_size,
        git_status,
        branch,
        remote_url,
        has_remote,
        commit_count,
        is_dirty,
        artifacts,
        health_score,
        health_grade,
    })
}

fn count_commits(repo: &git2::Repository, max: usize) -> usize {
    let mut revwalk = match repo.revwalk() {
        Ok(rw) => rw,
        Err(_) => return 0,
    };
    if revwalk.push_head().is_err() {
        return 0;
    }
    revwalk.take(max).filter(|r| r.is_ok()).count()
}

/// Quick discovery: find root directories that contain git repos
pub fn discover_project_dirs() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_default();
    let candidates = [
        "Projects",
        "Developer",
        "Code",
        "repos",
        "src",
        "workspace",
        "dev",
        "git",
        "work",
    ];

    let mut found = Vec::new();
    for name in &candidates {
        let path = home.join(name);
        if path.exists() && path.is_dir() {
            // Check if it contains at least one .git dir (depth 2)
            let has_repos = walkdir::WalkDir::new(&path)
                .max_depth(2)
                .into_iter()
                .filter_map(|e| e.ok())
                .any(|e| e.file_type().is_dir() && e.file_name() == ".git");
            if has_repos {
                found.push(path);
            }
        }
    }

    // Also check home dir directly (some people have repos at ~/)
    let home_has_repos = walkdir::WalkDir::new(&home)
        .max_depth(1)
        .into_iter()
        .filter_map(|e| e.ok())
        .any(|e| e.file_type().is_dir() && e.file_name() == ".git");
    if home_has_repos {
        found.push(home);
    }

    found
}
