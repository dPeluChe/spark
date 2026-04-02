//! Git history scanner: detect secrets in past commits using git2.
//!
//! Walks the commit history and scans diffs for secrets that may have been
//! committed and later removed. Uses the same regex patterns as secret_scanner.

use std::path::Path;
use std::collections::HashSet;
use super::secret_scanner::{SecretFinding, FindingCategory, FindingContext, Severity};

/// Maximum commits to scan (prevents very long scans on large repos)
const MAX_COMMITS: usize = 500;

/// A finding from git history includes commit metadata
#[derive(Debug, Clone)]
pub struct HistoryFinding {
    pub finding: SecretFinding,
    pub commit_sha: String,
    pub commit_msg: String,
    pub author: String,
    pub date: String,
}

/// Scan git history for secrets in past commits.
/// Returns findings from diffs that contain secret patterns.
pub fn scan_history(repo_path: &Path) -> Vec<HistoryFinding> {
    let repo = match git2::Repository::open(repo_path) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    let mut revwalk = match repo.revwalk() {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    if revwalk.push_head().is_err() {
        return Vec::new();
    }
    revwalk.set_sorting(git2::Sort::TIME).ok();

    let project_name = repo_path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut findings = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for (count, oid) in revwalk.filter_map(|r| r.ok()).enumerate() {
        if count >= MAX_COMMITS { break; }

        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let tree = match commit.tree() {
            Ok(t) => t,
            Err(_) => continue,
        };

        // Diff against parent (or empty tree for first commit)
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
        let diff = match repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let commit_sha = format!("{}", oid);
        let short_sha = &commit_sha[..7.min(commit_sha.len())];
        let commit_msg = commit.summary().unwrap_or("").to_string();
        let author = commit.author();
        let author_name = author.name().unwrap_or("unknown").to_string();
        let date = chrono::DateTime::from_timestamp(author.when().seconds(), 0)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default();

        // Scan each diff hunk for added lines
        let _ = diff.foreach(
            &mut |_delta, _progress| true,
            None,
            None,
            Some(&mut |_delta, _hunk, line| {
                // Only scan added lines
                if line.origin() != '+' { return true; }

                let content = match std::str::from_utf8(line.content()) {
                    Ok(s) => s.trim(),
                    Err(_) => return true,
                };

                if content.is_empty() || content.len() < 8 { return true; }

                // Run secret patterns on the line
                for matched in check_line_patterns(content) {
                    let fingerprint = format!("{}:{}:{}", matched.0, short_sha, &matched.2[..matched.2.len().min(20)]);
                    if seen.contains(&fingerprint) { continue; }
                    seen.insert(fingerprint);

                    let file_path = _delta.new_file().path()
                        .map(|p| repo_path.join(p))
                        .unwrap_or_default();

                    findings.push(HistoryFinding {
                        finding: SecretFinding {
                            file_path,
                            line_number: line.new_lineno().unwrap_or(0) as usize,
                            category: matched.0.clone(),
                            severity: matched.1,
                            context: FindingContext::SourceCode,
                            description: format!("{} (in commit {})", matched.3, short_sha),
                            redacted_match: matched.2.clone(),
                            project_name: project_name.clone(),
                            project_path: repo_path.to_path_buf(),
                        },
                        commit_sha: short_sha.to_string(),
                        commit_msg: commit_msg.clone(),
                        author: author_name.clone(),
                        date: date.clone(),
                    });
                }
                true
            }),
        );
    }

    findings
}

/// Check a single line against secret patterns (reuses patterns from secret_scanner).
fn check_line_patterns(line: &str) -> Vec<(FindingCategory, Severity, String, String)> {
    use super::secret_scanner::{API_KEY_PATTERNS, PRIVATE_KEY_CONTENT};
    use super::common;

    if common::is_likely_false_positive(line) {
        return Vec::new();
    }

    let mut results = Vec::new();
    for (pattern, desc) in API_KEY_PATTERNS.iter() {
        if let Some(m) = pattern.find(line) {
            results.push((FindingCategory::ApiKey, Severity::Critical, common::redact(m.as_str()), desc.to_string()));
        }
    }
    if PRIVATE_KEY_CONTENT.is_match(line) {
        results.push((FindingCategory::PrivateKey, Severity::Critical, "-----BEGIN PRIVATE KEY-----".into(), "Private Key".into()));
    }
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_line_patterns_aws() {
        let results = check_line_patterns("export AWS_KEY=AKIAIOSFODNN7TESTING1");
        assert!(!results.is_empty());
        assert_eq!(results[0].0, FindingCategory::ApiKey);
    }

    #[test]
    fn test_check_line_patterns_false_positive() {
        let results = check_line_patterns("api_key = 'your_api_key_here'");
        assert!(results.is_empty());
    }

    #[test]
    fn test_check_line_patterns_private_key() {
        let results = check_line_patterns("-----BEGIN RSA PRIVATE KEY-----");
        assert!(!results.is_empty());
        assert_eq!(results[0].0, FindingCategory::PrivateKey);
    }

    #[test]
    fn test_scan_history_nonexistent() {
        let findings = scan_history(Path::new("/tmp/nonexistent-repo"));
        assert!(findings.is_empty());
    }

    #[test]
    fn test_scan_history_no_panic() {
        // Scan our own repo — should not panic
        let findings = scan_history(Path::new("."));
        let _ = findings; // just verify no crash
    }
}
