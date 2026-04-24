//! Security scanner: detect exposed secrets, credentials, and sensitive files.
//!
//! Walks a directory recursively, checks each file by name (sensitive filenames,
//! private key extensions, `.env`) and by content (API keys, JWT, embedded
//! passwords in URLs, generic `password = "..."`), then groups findings by
//! project.

mod content;
mod context;
mod filename;
mod patterns;

pub use patterns::{API_KEY_PATTERNS, PRIVATE_KEY_CONTENT};

use super::common;
use std::path::{Path, PathBuf};

/// Severity of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Critical,
    Warning,
    Info,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Critical => write!(f, "CRITICAL"),
            Severity::Warning => write!(f, "WARNING"),
            Severity::Info => write!(f, "INFO"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FindingCategory {
    ApiKey,
    Credential,
    SensitiveFile,
    EmbeddedPassword,
    EnvFile,
    PrivateKey,
}

impl std::fmt::Display for FindingCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FindingCategory::ApiKey => write!(f, "API Key"),
            FindingCategory::Credential => write!(f, "Credential"),
            FindingCategory::SensitiveFile => write!(f, "Sensitive File"),
            FindingCategory::EmbeddedPassword => write!(f, "Embedded Password"),
            FindingCategory::EnvFile => write!(f, "Env File"),
            FindingCategory::PrivateKey => write!(f, "Private Key"),
        }
    }
}

/// Where the finding was located. Used to adjust severity — findings in tests
/// or docs are downgraded.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum FindingContext {
    SourceCode,
    Config,
    Test,
    Documentation,
    BuildArtifact,
}

impl std::fmt::Display for FindingContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FindingContext::SourceCode => write!(f, "Source Code"),
            FindingContext::Config => write!(f, "Config"),
            FindingContext::Test => write!(f, "Test"),
            FindingContext::Documentation => write!(f, "Docs"),
            FindingContext::BuildArtifact => write!(f, "Build"),
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SecretFinding {
    pub file_path: PathBuf,
    pub line_number: usize,
    pub category: FindingCategory,
    pub severity: Severity,
    pub context: FindingContext,
    pub description: String,
    pub redacted_match: String,
    pub project_name: String,
    pub project_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct AuditResult {
    pub project_name: String,
    pub project_path: PathBuf,
    pub findings: Vec<SecretFinding>,
    pub critical_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

pub fn scan_directory(path: &Path) -> Vec<AuditResult> {
    scan_directory_with_progress(path, None)
}

pub fn scan_directory_with_progress(
    path: &Path,
    on_progress: Option<&dyn Fn(usize)>,
) -> Vec<AuditResult> {
    let mut findings: Vec<SecretFinding> = Vec::new();
    let mut files_scanned = 0usize;
    let ignore_patterns = common::load_ignore_patterns(path);

    for entry in walkdir::WalkDir::new(path)
        .max_depth(8)
        .into_iter()
        .filter_entry(|e| !should_skip_dir(e))
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        files_scanned += 1;
        if let Some(cb) = &on_progress {
            cb(files_scanned);
        }
        let file_path = entry.path();

        if common::is_ignored(file_path, path, &ignore_patterns) {
            continue;
        }

        if common::BINARY_EXT.contains(
            &file_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase()
                .as_str(),
        ) {
            continue;
        }

        if let Ok(meta) = std::fs::metadata(file_path) {
            if meta.len() > common::MAX_FILE_SIZE {
                continue;
            }
        }

        let (project_name, project_path) = find_project(file_path, path);

        findings.extend(filename::check_filename(
            file_path,
            &project_name,
            &project_path,
        ));
        findings.extend(content::check_content(
            file_path,
            &project_name,
            &project_path,
        ));
    }

    group_by_project(findings)
}

fn should_skip_dir(entry: &walkdir::DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    common::SKIP_DIRS.contains(&name.as_ref())
}

fn find_project(file_path: &Path, scan_root: &Path) -> (String, PathBuf) {
    let mut check = file_path.parent().unwrap_or(scan_root).to_path_buf();
    loop {
        if check.join(".git").exists() {
            let name = check
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| check.display().to_string());
            return (name, check);
        }
        if check == scan_root || !check.pop() {
            break;
        }
    }
    let name = scan_root
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into());
    (name, scan_root.to_path_buf())
}

fn group_by_project(findings: Vec<SecretFinding>) -> Vec<AuditResult> {
    use std::collections::BTreeMap;

    let mut groups: BTreeMap<PathBuf, (String, Vec<SecretFinding>)> = BTreeMap::new();
    for f in findings {
        let entry = groups
            .entry(f.project_path.clone())
            .or_insert_with(|| (f.project_name.clone(), Vec::new()));
        entry.1.push(f);
    }

    let mut results: Vec<AuditResult> = groups
        .into_iter()
        .map(|(path, (name, mut findings))| {
            findings.sort_by(|a, b| {
                a.context
                    .cmp(&b.context)
                    .then(a.severity.cmp(&b.severity))
                    .then(a.file_path.cmp(&b.file_path))
                    .then(a.line_number.cmp(&b.line_number))
            });
            let critical = findings
                .iter()
                .filter(|f| f.severity == Severity::Critical)
                .count();
            let warning = findings
                .iter()
                .filter(|f| f.severity == Severity::Warning)
                .count();
            let info = findings
                .iter()
                .filter(|f| f.severity == Severity::Info)
                .count();
            AuditResult {
                project_name: name,
                project_path: path,
                findings,
                critical_count: critical,
                warning_count: warning,
                info_count: info,
            }
        })
        .collect();

    // Most critical projects first
    results.sort_by(|a, b| {
        b.critical_count
            .cmp(&a.critical_count)
            .then(b.warning_count.cmp(&a.warning_count))
    });
    results
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_aws_key_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.txt");
        fs::write(&file, "aws_key = AKIAIOSFODNN7TESTING1\n").unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(all_findings.iter().any(|f| f.description.contains("AWS")));
    }

    #[test]
    fn test_github_token_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("script.sh");
        fs::write(
            &file,
            "GITHUB_TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(all_findings
            .iter()
            .any(|f| f.description.contains("GitHub")));
    }

    #[test]
    fn test_env_file_is_info() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(".env");
        fs::write(&file, "PORT=3000\n").unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(all_findings
            .iter()
            .any(|f| f.severity == Severity::Info && f.category == FindingCategory::EnvFile));
    }

    #[test]
    fn test_env_example_not_flagged() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(".env.example");
        fs::write(&file, "API_KEY=your_key_here\n").unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(!all_findings
            .iter()
            .any(|f| f.category == FindingCategory::EnvFile));
    }

    #[test]
    fn test_pem_file_is_critical() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("server.pem");
        fs::write(&file, "cert data").unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(
            all_findings
                .iter()
                .any(|f| f.severity == Severity::Critical
                    && f.category == FindingCategory::PrivateKey)
        );
    }

    #[test]
    fn test_false_positive_filter() {
        assert!(common::is_likely_false_positive(
            "api_key = 'your_api_key_here'"
        ));
        assert!(common::is_likely_false_positive("password = 'changeme'"));
        assert!(common::is_likely_false_positive(
            "# example: token = sk-xxx"
        ));
        assert!(!common::is_likely_false_positive(
            "AWS_SECRET=wJalrXUtnFEMI/K7MDENG/bPxRfi"
        ));
    }

    #[test]
    fn test_redact() {
        assert_eq!(common::redact("AKIAIOSFODNN7EXAMPLE"), "AKIA****MPLE");
        assert_eq!(common::redact("short"), "shor****");
    }

    #[test]
    fn test_skip_binary_ext() {
        assert!(common::BINARY_EXT.contains(&"png"));
        assert!(common::BINARY_EXT.contains(&"gz"));
        assert!(!common::BINARY_EXT.contains(&"json"));
        assert!(!common::BINARY_EXT.contains(&"sh"));
    }

    #[test]
    fn test_skip_node_modules() {
        let entry = walkdir::WalkDir::new(".")
            .into_iter()
            .filter_map(|e| e.ok())
            .next()
            .unwrap();
        let _ = should_skip_dir(&entry);
    }

    #[test]
    fn test_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let results = scan_directory(dir.path());
        assert!(results.is_empty());
    }

    #[test]
    fn test_url_with_password() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("deploy.sh");
        fs::write(
            &file,
            "curl https://admin:supersecret@api.internal.io/data\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(all_findings
            .iter()
            .any(|f| f.category == FindingCategory::EmbeddedPassword));
    }

    #[test]
    fn test_generic_secret() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.py");
        fs::write(&file, "password = 'my_super_secret_password_123'\n").unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(all_findings
            .iter()
            .any(|f| f.category == FindingCategory::Credential));
    }

    #[test]
    fn test_private_key_content() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("key.txt");
        fs::write(
            &file,
            "-----BEGIN RSA PRIVATE KEY-----\nMIIE...\n-----END RSA PRIVATE KEY-----\n",
        )
        .unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(all_findings
            .iter()
            .any(|f| f.category == FindingCategory::PrivateKey));
    }
}
