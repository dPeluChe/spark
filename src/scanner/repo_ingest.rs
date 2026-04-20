//! Repository ingest: generate LLM-ready context files via trs.
//!
//! trs ingest is the sole backend — budget-aware, git-aware, agent-friendly.
//! Output stored in ~/.config/spark/ingest/<host>/<owner>/<name>.md
//!
//! Install trs: npm install -g @dpeluche/trs

use std::path::{Path, PathBuf};
use std::process::Command;

/// Options forwarded to `trs ingest`
#[derive(Default)]
pub struct IngestOptions {
    /// Aggressive compression: trs -l aggressive (~93% reduction)
    pub compress: bool,
    /// Token budget constraint, e.g. "32k", "128k"
    pub budget: Option<String>,
    /// Only uncommitted/modified files
    pub changed: bool,
    /// Only files changed since a git ref, e.g. "HEAD~5"
    pub since: Option<String>,
    /// Dependency graph only — no file content
    pub deps: bool,
    /// Reuse cached digest if HEAD unchanged (skip regeneration)
    pub fresh: bool,
}

pub fn is_trs_available() -> bool {
    Command::new("trs")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn ingest_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("spark")
        .join("ingest")
}

pub fn ingest_path(host: &str, owner: &str, name: &str) -> PathBuf {
    ingest_dir()
        .join(host)
        .join(owner)
        .join(format!("{}.md", name))
}

#[allow(dead_code)]
pub fn has_ingest(host: &str, owner: &str, name: &str) -> bool {
    ingest_path(host, owner, name).exists()
}

pub fn ingest_info(host: &str, owner: &str, name: &str) -> Option<IngestInfo> {
    let path = ingest_path(host, owner, name);
    let meta = std::fs::metadata(&path).ok()?;
    let age = meta
        .modified()
        .ok()
        .and_then(|t| t.elapsed().ok())
        .map(|d| d.as_secs());
    Some(IngestInfo {
        path,
        size: meta.len(),
        age_secs: age,
    })
}

pub struct IngestInfo {
    pub path: PathBuf,
    pub size: u64,
    pub age_secs: Option<u64>,
}

impl IngestInfo {
    pub fn age_display(&self) -> String {
        match self.age_secs {
            Some(s) if s < 3600 => format!("{}m ago", s / 60),
            Some(s) if s < 86400 => format!("{}h ago", s / 3600),
            Some(s) => format!("{}d ago", s / 86400),
            None => "unknown".into(),
        }
    }
}

/// Generate ingest for a repo via trs.
/// Returns the path where the digest was written.
pub fn generate_ingest(
    repo_path: &Path,
    host: &str,
    owner: &str,
    name: &str,
    opts: &IngestOptions,
) -> Result<PathBuf, String> {
    if !is_trs_available() {
        return Err(
            "trs not found. Install it:\n  npm install -g @dpeluche/trs\n  or: curl -fsSL https://raw.githubusercontent.com/dPeluChe/trs/main/scripts/install.sh | sh".to_string()
        );
    }

    let output_path = ingest_path(host, owner, name);
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create ingest directory: {}", e))?;
    }

    let mut args = vec!["ingest".to_string()];

    // Write directly to spark's path — no shadow save in ~/.trs/ingest/
    args.push("-o".into());
    args.push(output_path.display().to_string());

    if let Some(ref budget) = opts.budget {
        args.push("--budget".into());
        args.push(budget.clone());
    }
    if opts.changed {
        args.push("--changed".into());
    }
    if let Some(ref since) = opts.since {
        args.push("--since".into());
        args.push(since.clone());
    }
    if opts.deps {
        args.push("--deps".into());
    }
    if opts.fresh {
        args.push("--fresh".into());
    }
    if opts.compress {
        args.push("-l".into());
        args.push("aggressive".into());
    }

    let result = Command::new("trs")
        .args(&args)
        .current_dir(repo_path)
        .output()
        .map_err(|e| format!("Failed to run trs: {}", e))?;

    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        return Err(format!("trs ingest failed: {}", stderr.trim()));
    }

    // trs -o writes the file and returns the path on stdout
    Ok(output_path)
}

/// List all existing ingest files
pub fn list_ingests() -> Vec<(String, String, String, IngestInfo)> {
    let base = ingest_dir();
    if !base.exists() {
        return Vec::new();
    }

    let mut results = Vec::new();

    for host_entry in std::fs::read_dir(&base).into_iter().flatten().flatten() {
        if !host_entry.path().is_dir() {
            continue;
        }
        let host = host_entry.file_name().to_string_lossy().to_string();

        for owner_entry in std::fs::read_dir(host_entry.path())
            .into_iter()
            .flatten()
            .flatten()
        {
            if !owner_entry.path().is_dir() {
                continue;
            }
            let owner = owner_entry.file_name().to_string_lossy().to_string();

            for file_entry in std::fs::read_dir(owner_entry.path())
                .into_iter()
                .flatten()
                .flatten()
            {
                let path = file_entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let name = path
                    .file_stem()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                if let Some(info) = ingest_info(&host, &owner, &name) {
                    results.push((host.clone(), owner.clone(), name, info));
                }
            }
        }
    }

    results.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_path() {
        let path = ingest_path("github.com", "user", "repo");
        assert!(path.display().to_string().contains("github.com/user"));
        assert!(path.display().to_string().ends_with("repo.md"));
    }

    #[test]
    fn test_has_ingest_nonexistent() {
        assert!(!has_ingest("nonexistent.com", "nobody", "nothing"));
    }

    #[test]
    fn test_list_ingests_no_panic() {
        let _ = list_ingests();
    }
}
