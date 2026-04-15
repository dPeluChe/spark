//! Repository ingest: generate LLM-ready context files using repomix.
//!
//! After clone/pull, generates a single .md file with the full repo content
//! optimized for LLM consumption. Stored in ~/.config/spark/ingest/ mirroring
//! the repos_root structure.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Get the ingest directory path
fn ingest_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("spark")
        .join("ingest")
}

/// Get the ingest file path for a repo (mirrors repos_root structure)
pub fn ingest_path(host: &str, owner: &str, name: &str) -> PathBuf {
    ingest_dir().join(host).join(owner).join(format!("{}.md", name))
}

#[allow(dead_code)]
pub fn has_ingest(host: &str, owner: &str, name: &str) -> bool {
    ingest_path(host, owner, name).exists()
}

/// Get ingest file info (exists, size, age)
pub fn ingest_info(host: &str, owner: &str, name: &str) -> Option<IngestInfo> {
    let path = ingest_path(host, owner, name);
    let meta = std::fs::metadata(&path).ok()?;
    let age = meta.modified().ok()
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

/// Check if npx is available (repomix runs via npx)
pub fn is_npx_available() -> bool {
    Command::new("npx").arg("--version").output()
        .map(|o| o.status.success()).unwrap_or(false)
}

/// Generate ingest for a repo
pub fn generate_ingest(
    repo_path: &Path,
    host: &str,
    owner: &str,
    name: &str,
    compress: bool,
) -> Result<PathBuf, String> {
    let output_path = ingest_path(host, owner, name);

    // Create parent dirs
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create ingest directory: {}", e))?;
    }

    // Build repomix command
    let output_str = output_path.display().to_string();
    let mut args = vec![
        "repomix@latest".to_string(),
        "--style".into(), "markdown".into(),
        "--output".into(), output_str.clone(),
    ];

    if compress {
        args.push("--compress".into());
    }

    // Add the repo path as the target
    args.push(repo_path.display().to_string());

    // Try npx first, then direct repomix
    let result = Command::new("npx")
        .args(&args)
        .current_dir(repo_path)
        .output();

    match result {
        Ok(output) if output.status.success() => Ok(output_path),
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("repomix failed: {}", stderr.trim()))
        }
        Err(e) => Err(format!("Failed to run repomix: {}", e)),
    }
}

/// List all existing ingest files
pub fn list_ingests() -> Vec<(String, String, String, IngestInfo)> {
    let base = ingest_dir();
    if !base.exists() { return Vec::new(); }

    let mut results = Vec::new();

    // Walk host/owner/name.md structure
    let hosts = match std::fs::read_dir(&base) {
        Ok(e) => e,
        Err(_) => return results,
    };

    for host_entry in hosts.filter_map(|e| e.ok()) {
        if !host_entry.path().is_dir() { continue; }
        let host = host_entry.file_name().to_string_lossy().to_string();

        let owners = match std::fs::read_dir(host_entry.path()) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for owner_entry in owners.filter_map(|e| e.ok()) {
            if !owner_entry.path().is_dir() { continue; }
            let owner = owner_entry.file_name().to_string_lossy().to_string();

            let files = match std::fs::read_dir(owner_entry.path()) {
                Ok(e) => e,
                Err(_) => continue,
            };

            for file_entry in files.filter_map(|e| e.ok()) {
                let path = file_entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") { continue; }

                let name = path.file_stem()
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
        assert!(path.display().to_string().contains("github.com"));
        assert!(path.display().to_string().contains("user"));
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
