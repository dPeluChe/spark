use std::path::PathBuf;
use chrono::{DateTime, Utc};
use crate::scanner::space_analyzer::ArtifactInfo;

/// Status of git in a repository
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum RepoGitStatus {
    Clean,
    Dirty {
        untracked: usize,
        modified: usize,
        staged: usize,
    },
    NotARepo,
}

/// Detected workspace type
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceType {
    None,
    Npm,        // package.json with "workspaces"
    Pnpm,       // pnpm-workspace.yaml
    Turborepo,  // turbo.json
    Nx,         // nx.json
    Lerna,      // lerna.json
    Cargo,      // Cargo.toml with [workspace]
    GoWork,     // go.work
}

impl std::fmt::Display for WorkspaceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceType::None => write!(f, ""),
            WorkspaceType::Npm => write!(f, "npm workspace"),
            WorkspaceType::Pnpm => write!(f, "pnpm workspace"),
            WorkspaceType::Turborepo => write!(f, "turborepo"),
            WorkspaceType::Nx => write!(f, "nx"),
            WorkspaceType::Lerna => write!(f, "lerna"),
            WorkspaceType::Cargo => write!(f, "cargo workspace"),
            WorkspaceType::GoWork => write!(f, "go workspace"),
        }
    }
}

/// Full information about a discovered repository
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RepoInfo {
    pub path: PathBuf,
    pub name: String,
    pub group: String,
    pub is_container: bool,
    pub child_repo_count: usize,
    pub workspace: WorkspaceType,
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

        // Derive group name from scan root
        let group = dir.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| dir.display().to_string());

        for entry in walkdir::WalkDir::new(dir)
            .max_depth(max_depth)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                // Skip node_modules, .git internals, venvs during walk
                !matches!(name.as_ref(), "node_modules" | ".venv" | "venv" | "__pycache__")
            })
            .filter_map(|e| e.ok())
        {
            dirs_scanned += 1;

            if entry.file_type().is_dir() && entry.file_name() == ".git" {
                if let Some(parent) = entry.path().parent() {
                    if let Some(mut repo) = analyze_repo(parent) {
                        // Group = first 2 levels of relative path from scan root
                        let rel = parent.strip_prefix(dir).unwrap_or(parent);
                        let components: Vec<String> = rel.components()
                            .map(|c| c.as_os_str().to_string_lossy().to_string())
                            .collect();

                        repo.group = if components.len() <= 1 {
                            // Repo is direct child or 1 level deep
                            group.clone()
                        } else {
                            // Use up to 2 parent levels as group
                            let depth = (components.len() - 1).min(2);
                            components[..depth].join("/")
                        };

                        let _ = tx.send(ScanProgressMsg {
                            repos_found: repos.len() + 1,
                            dirs_scanned,
                            current_dir: parent.display().to_string(),
                        });
                        repos.push(repo);
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

    // Skip commit counting for speed (use 0)
    let commit_count = 0;

    // Artifacts (fast: just checks existence, sizes via dir_size)
    let artifacts = find_artifacts(path);
    let artifact_size: u64 = artifacts.iter().map(|a| a.size).sum();

    // Total size: .git size + artifact size (fast approximation)
    let git_size = crate::utils::fs::dir_size(&path.join(".git"));
    let total_size = git_size + artifact_size;

    // Last modified
    let last_modified = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());

    // Health score
    let (health_score, health_grade) =
        calculate_health(last_commit_date, last_modified, has_remote, is_dirty, artifact_size);

    // Detect if this is a container (has child repos)
    let child_count = count_git_repos(path, 2);
    let is_container = child_count > 1;

    // Detect workspace type
    let workspace = detect_workspace(path);

    Some(RepoInfo {
        path: path.to_path_buf(),
        name,
        group: String::new(), // set by scan_directories
        is_container,
        child_repo_count: if is_container { child_count - 1 } else { 0 },
        workspace,
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

/// Detect workspace type from project files
fn detect_workspace(path: &std::path::Path) -> WorkspaceType {
    // Turborepo
    if path.join("turbo.json").exists() {
        return WorkspaceType::Turborepo;
    }
    // Nx
    if path.join("nx.json").exists() {
        return WorkspaceType::Nx;
    }
    // Lerna
    if path.join("lerna.json").exists() {
        return WorkspaceType::Lerna;
    }
    // pnpm workspace
    if path.join("pnpm-workspace.yaml").exists() {
        return WorkspaceType::Pnpm;
    }
    // npm workspace (check package.json for "workspaces" field)
    if let Ok(content) = std::fs::read_to_string(path.join("package.json")) {
        if content.contains("\"workspaces\"") {
            return WorkspaceType::Npm;
        }
    }
    // Cargo workspace
    if let Ok(content) = std::fs::read_to_string(path.join("Cargo.toml")) {
        if content.contains("[workspace]") {
            return WorkspaceType::Cargo;
        }
    }
    // Go workspace
    if path.join("go.work").exists() {
        return WorkspaceType::GoWork;
    }
    WorkspaceType::None
}

/// Quick scan of direct child repos in a container (for detail view)
pub fn scan_container_children(container_path: &std::path::Path) -> Vec<RepoInfo> {
    let mut children = Vec::new();
    let entries = match std::fs::read_dir(container_path) {
        Ok(e) => e,
        Err(_) => return children,
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() || !path.join(".git").exists() {
            continue;
        }
        if let Some(repo) = analyze_repo(&path) {
            children.push(repo);
        }
    }

    children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    children
}

/// Directories to always skip when discovering project dirs
const SKIP_DIRS: &[&str] = &[
    "Library", "Applications", "Music", "Movies", "Pictures",
    "Public", "Downloads", "Documents",
];

/// A discovered directory with its repo count
#[derive(Debug)]
pub struct DiscoveredDir {
    pub path: PathBuf,
    pub repo_count: usize,
}

/// Quick discovery: suggest root directories that contain git repos.
/// Only returns **parent** dirs (not individual repos), with repo counts.
/// Scans each direct child of ~/ checking if it contains repos inside.
pub fn discover_project_dirs() -> Vec<DiscoveredDir> {
    let home = dirs::home_dir().unwrap_or_default();
    let mut found = Vec::new();

    let children = match std::fs::read_dir(&home) {
        Ok(entries) => entries,
        Err(_) => return found,
    };

    for entry in children.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || SKIP_DIRS.contains(&name.as_str()) {
            continue;
        }
        let repo_count = count_git_repos(&path, 4);
        if repo_count > 0 {
            found.push(DiscoveredDir { path, repo_count });
        }
    }

    // Sort by repo count descending (most repos first)
    found.sort_by(|a, b| b.repo_count.cmp(&a.repo_count));
    found
}

/// Count repos in a directory (public helper for manual add)
pub fn count_repos_in(path: &std::path::Path) -> usize {
    count_git_repos(path, 4)
}

fn count_git_repos(path: &std::path::Path, depth: usize) -> usize {
    walkdir::WalkDir::new(path)
        .max_depth(depth)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !name.starts_with('.') || name == ".git"
        })
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_dir() && e.file_name() == ".git")
        .count()
}

