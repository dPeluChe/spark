//! Repository ingest: generate LLM-ready context files via trs.
//!
//! trs owns digest generation AND storage — SPARK delegates both, then
//! enriches the catalog with fleet-level info (which managed repos have
//! no digest yet, which are stale vs repo status cache).
//!
//! Storage: ~/.trs/ingest/<owner>/<name>.md (shared with TRS, single
//! source of truth). Previously SPARK maintained a parallel catalog at
//! ~/.config/spark/ingest — removed to eliminate duplication.
//!
//! See docs/dev/TRS_INTEGRATION.md for the full architectural rationale.
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

/// Shared storage location with TRS. Resolves `$HOME/.trs/ingest`.
fn ingest_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".trs")
        .join("ingest")
}

pub fn ingest_path(owner: &str, name: &str) -> PathBuf {
    ingest_dir().join(owner).join(format!("{}.md", name))
}

pub fn ingest_info(owner: &str, name: &str) -> Option<IngestInfo> {
    let path = ingest_path(owner, name);
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

/// Run `trs ingest` inside the repo directory. TRS auto-detects owner/name
/// from the git remote and writes to its own storage (~/.trs/ingest).
/// Returns the resolved path where the digest was written.
pub fn generate_ingest(
    repo_path: &Path,
    owner: &str,
    name: &str,
    opts: &IngestOptions,
) -> Result<PathBuf, String> {
    if !is_trs_available() {
        return Err(
            "trs not found. Install it:\n  npm install -g @dpeluche/trs\n  or: curl -fsSL https://raw.githubusercontent.com/dPeluChe/trs/main/scripts/install.sh | sh".to_string()
        );
    }

    let mut args = vec!["ingest".to_string()];

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

    Ok(ingest_path(owner, name))
}

/// List all existing digests from TRS's storage.
/// Returns (owner, name, info) tuples sorted by owner then name.
pub fn list_ingests() -> Vec<(String, String, IngestInfo)> {
    let base = ingest_dir();
    if !base.exists() {
        return Vec::new();
    }

    let mut results = Vec::new();

    for owner_entry in std::fs::read_dir(&base).into_iter().flatten().flatten() {
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
            if let Some(info) = ingest_info(&owner, &name) {
                results.push((owner.clone(), name, info));
            }
        }
    }

    results.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_path() {
        let path = ingest_path("user", "repo");
        let s = path.display().to_string();
        assert!(s.contains(".trs"));
        assert!(s.contains("ingest"));
        assert!(s.contains("user"));
        assert!(s.ends_with("repo.md"));
    }

    #[test]
    fn test_ingest_path_no_host() {
        // TRS layout is owner/name.md — no host segment
        let path = ingest_path("user", "repo");
        let components: Vec<_> = path
            .components()
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .collect();
        assert!(!components.iter().any(|c| c == "github.com"));
    }

    #[test]
    fn test_list_ingests_no_panic() {
        let _ = list_ingests();
    }
}
