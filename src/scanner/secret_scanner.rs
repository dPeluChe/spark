//! Security scanner: detect exposed secrets, credentials, and sensitive files.
//!
//! Scans directories recursively for API keys, tokens, passwords, and files
//! that should not be committed to version control.

use std::path::{Path, PathBuf};
use once_cell::sync::Lazy;
use regex::Regex;
use super::common;


/// Severity of a finding
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

/// Category of finding
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

/// Where the finding was located (affects how it's displayed)
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

/// A single security finding
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

/// Aggregated results for a project
#[derive(Debug, Clone)]
pub struct AuditResult {
    pub project_name: String,
    pub project_path: PathBuf,
    pub findings: Vec<SecretFinding>,
    pub critical_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

// --- Regex patterns ---

static AWS_KEY: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:AKIA|ASIA|ABIA|ACCA)[0-9A-Z]{16}").unwrap());
static AWS_SECRET: Lazy<Regex> = Lazy::new(|| Regex::new(r#"(?i)aws[_\-]?secret[_\-]?access[_\-]?key\s*[=:]\s*['"]?([A-Za-z0-9/+=]{40})['"]?"#).unwrap());
static GITHUB_TOKEN: Lazy<Regex> = Lazy::new(|| Regex::new(r"ghp_[A-Za-z0-9]{36}").unwrap());
static GITHUB_OAUTH: Lazy<Regex> = Lazy::new(|| Regex::new(r"gho_[A-Za-z0-9]{36}").unwrap());
static ANTHROPIC_KEY: Lazy<Regex> = Lazy::new(|| Regex::new(r"sk-ant-api03-[A-Za-z0-9\-_]{80,}").unwrap());
static OPENAI_KEY: Lazy<Regex> = Lazy::new(|| Regex::new(r"sk-[A-Za-z0-9]{20}T3BlbkFJ[A-Za-z0-9]{20}").unwrap());
static GOOGLE_API: Lazy<Regex> = Lazy::new(|| Regex::new(r"AIzaSy[A-Za-z0-9\-_]{33}").unwrap());
static SENDGRID_KEY: Lazy<Regex> = Lazy::new(|| Regex::new(r"SG\.[A-Za-z0-9\-_]{22,}").unwrap());
static SLACK_TOKEN: Lazy<Regex> = Lazy::new(|| Regex::new(r"xox[bpors]-[0-9]{10,13}-[0-9a-zA-Z\-]{10,}").unwrap());
static STRIPE_KEY: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:sk|pk)_(?:test|live)_[A-Za-z0-9]{20,}").unwrap());
static TWILIO_KEY: Lazy<Regex> = Lazy::new(|| Regex::new(r"SK[a-f0-9]{32}").unwrap());
static JWT_TOKEN: Lazy<Regex> = Lazy::new(|| Regex::new(r"eyJ[A-Za-z0-9\-_]+\.eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+").unwrap());
static URL_WITH_PASS: Lazy<Regex> = Lazy::new(|| Regex::new(r"[a-z]+://[^:]+:[^@\s]{3,}@[^\s]+").unwrap());
static GENERIC_SECRET: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)(?:password|passwd|secret|token|api[_\-]?key|apikey|auth[_\-]?token|access[_\-]?key)\s*[=:]\s*['"]([^'"]{8,})['"]"#).unwrap()
});
static NPM_TOKEN: Lazy<Regex> = Lazy::new(|| Regex::new(r"//registry\.npmjs\.org/:_authToken=\S+").unwrap());
pub static PRIVATE_KEY_CONTENT: Lazy<Regex> = Lazy::new(|| Regex::new(r"-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap());

/// API key patterns: (regex, description) — shared with history_scanner
pub static API_KEY_PATTERNS: Lazy<Vec<(&Regex, &str)>> = Lazy::new(|| vec![
    (&AWS_KEY, "AWS Access Key ID"),
    (&AWS_SECRET, "AWS Secret Access Key"),
    (&GITHUB_TOKEN, "GitHub Personal Access Token"),
    (&GITHUB_OAUTH, "GitHub OAuth Token"),
    (&ANTHROPIC_KEY, "Anthropic API Key"),
    (&OPENAI_KEY, "OpenAI API Key"),
    (&GOOGLE_API, "Google API Key"),
    (&SENDGRID_KEY, "SendGrid API Key"),
    (&SLACK_TOKEN, "Slack Token"),
    (&STRIPE_KEY, "Stripe API Key"),
    (&TWILIO_KEY, "Twilio API Key"),
    (&JWT_TOKEN, "JWT Token"),
    (&NPM_TOKEN, "NPM Auth Token"),
]);

/// Sensitive filenames (exact match or glob-like)
const SENSITIVE_FILES: &[(&str, Severity, &str)] = &[
    ("id_rsa", Severity::Critical, "SSH Private Key"),
    ("id_ecdsa", Severity::Critical, "SSH Private Key (ECDSA)"),
    ("id_ed25519", Severity::Critical, "SSH Private Key (Ed25519)"),
    ("id_dsa", Severity::Critical, "SSH Private Key (DSA)"),
    (".htpasswd", Severity::Critical, "Apache Password File"),
    ("credentials.json", Severity::Warning, "Credentials File"),
    ("service-account.json", Severity::Warning, "Service Account Credentials"),
    ("keystore.jks", Severity::Warning, "Java Keystore"),
    (".git-credentials", Severity::Critical, "Git Credentials"),
    (".netrc", Severity::Warning, "Netrc Credentials"),
];

/// Sensitive extensions
const SENSITIVE_EXTENSIONS: &[(&str, Severity, &str)] = &[
    ("pem", Severity::Critical, "PEM Certificate/Key"),
    ("key", Severity::Critical, "Private Key File"),
    ("p12", Severity::Warning, "PKCS#12 Certificate"),
    ("pfx", Severity::Warning, "PFX Certificate"),
    ("jks", Severity::Warning, "Java Keystore"),
    ("keystore", Severity::Warning, "Keystore File"),
];

/// Config files that may contain credentials
const CREDENTIAL_CONFIG_FILES: &[&str] = &[
    ".npmrc",
    ".pypirc",
    ".docker/config.json",
    ".terraformrc",
    "credentials.tfrc.json",
];

// Constants and shared utilities are in scanner::common

/// Scan a directory for secrets, returning results grouped by project.
pub fn scan_directory(path: &Path) -> Vec<AuditResult> {
    scan_directory_with_progress(path, None)
}

/// Scan with optional progress callback (called per file scanned)
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
        if let Some(cb) = &on_progress { cb(files_scanned); }
        let file_path = entry.path();

        // Skip ignored files
        if common::is_ignored(file_path, path, &ignore_patterns) { continue; }

        // Skip binary files by extension
        if common::BINARY_EXT.contains(&file_path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase().as_str()) {
            continue;
        }

        // Skip large files
        if let Ok(meta) = std::fs::metadata(file_path) {
            if meta.len() > common::MAX_FILE_SIZE {
                continue;
            }
        }

        let (project_name, project_path) = find_project(file_path, path);

        // Check filename patterns
        findings.extend(check_filename(file_path, &project_name, &project_path));

        // Check file content
        findings.extend(check_content(file_path, &project_name, &project_path));
    }

    // Group by project
    group_by_project(findings)
}

fn should_skip_dir(entry: &walkdir::DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    common::SKIP_DIRS.contains(&name.as_ref())
}

/// Determine context from file path
fn detect_context(path: &Path) -> FindingContext {
    let path_str = path.display().to_string().to_lowercase();
    let file_name = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();

    // Test files
    if file_name.contains(".test.") || file_name.contains(".spec.")
        || file_name.contains("_test.") || file_name.contains("_spec.")
        || path_str.contains("/tests/") || path_str.contains("/__tests__/")
        || path_str.contains("/test/") || path_str.contains("/spec/")
        || file_name.starts_with("test_") || file_name.starts_with("test-")
        || path_str.contains("/smoke/") || path_str.contains("/fixtures/")
    {
        return FindingContext::Test;
    }

    // Documentation
    if file_name.ends_with(".md") || file_name.ends_with(".rst")
        || file_name.ends_with(".txt") || file_name.ends_with(".adoc")
        || path_str.contains("/docs/") || path_str.contains("/documentation/")
    {
        return FindingContext::Documentation;
    }

    // Build artifacts
    if path_str.contains("/dist/") || path_str.contains("/build/")
        || path_str.contains("/dist-") || path_str.contains("/.output/")
    {
        return FindingContext::BuildArtifact;
    }

    // Config files
    if file_name.starts_with('.') || file_name.ends_with(".json")
        || file_name.ends_with(".toml") || file_name.ends_with(".yaml")
        || file_name.ends_with(".yml") || file_name.ends_with(".ini")
        || file_name.ends_with(".cfg") || file_name.ends_with(".conf")
        || path_str.contains("/config/") || path_str.contains("/scripts/")
        || file_name.contains("config") || file_name == "seed.ts"
        || file_name == "seed.js"
    {
        return FindingContext::Config;
    }

    FindingContext::SourceCode
}

/// Check if a URL contains a safe domain (not credentials)
fn is_safe_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    common::SAFE_URL_DOMAINS.iter().any(|domain| lower.contains(domain))
}

/// Find the project (nearest .git parent) for a file
fn find_project(file_path: &Path, scan_root: &Path) -> (String, PathBuf) {
    let mut check = file_path.parent().unwrap_or(scan_root).to_path_buf();
    loop {
        if check.join(".git").exists() {
            let name = check.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| check.display().to_string());
            return (name, check);
        }
        if check == scan_root || !check.pop() {
            break;
        }
    }
    // No git parent found — use scan root
    let name = scan_root.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".into());
    (name, scan_root.to_path_buf())
}

/// Check if the filename itself is sensitive
fn check_filename(path: &Path, project_name: &str, project_path: &Path) -> Vec<SecretFinding> {
    let mut findings = Vec::new();
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let ctx = detect_context(path);

    // Check exact filenames
    for (name, severity, desc) in SENSITIVE_FILES {
        if file_name.as_ref() == *name {
            findings.push(SecretFinding {
                file_path: path.to_path_buf(), line_number: 0,
                category: FindingCategory::SensitiveFile, severity: *severity,
                context: ctx.clone(), description: desc.to_string(),
                redacted_match: file_name.to_string(),
                project_name: project_name.to_string(), project_path: project_path.to_path_buf(),
            });
        }
    }

    // Check extensions
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        for (sensitive_ext, severity, desc) in SENSITIVE_EXTENSIONS {
            if ext.eq_ignore_ascii_case(sensitive_ext) {
                findings.push(SecretFinding {
                    file_path: path.to_path_buf(), line_number: 0,
                    category: FindingCategory::PrivateKey, severity: *severity,
                    context: ctx.clone(), description: desc.to_string(),
                    redacted_match: file_name.to_string(),
                    project_name: project_name.to_string(), project_path: project_path.to_path_buf(),
                });
            }
        }
    }

    // .env files — info level
    if (file_name == ".env" || file_name.starts_with(".env."))
        && !file_name.contains("example") && !file_name.contains("sample") && !file_name.contains("template")
    {
        findings.push(SecretFinding {
            file_path: path.to_path_buf(), line_number: 0,
            category: FindingCategory::EnvFile, severity: Severity::Info,
            context: FindingContext::Config,
            description: "Environment file (may contain secrets)".into(),
            redacted_match: file_name.to_string(),
            project_name: project_name.to_string(), project_path: project_path.to_path_buf(),
        });
    }

    // Known credential config files
    for config_file in CREDENTIAL_CONFIG_FILES {
        let config_name = Path::new(config_file).file_name().unwrap_or_default().to_string_lossy();
        if file_name.as_ref() == config_name.as_ref() {
            findings.push(SecretFinding {
                file_path: path.to_path_buf(), line_number: 0,
                category: FindingCategory::Credential, severity: Severity::Warning,
                context: FindingContext::Config,
                description: format!("Config file that may contain credentials ({})", config_file),
                redacted_match: file_name.to_string(),
                project_name: project_name.to_string(), project_path: project_path.to_path_buf(),
            });
        }
    }

    findings
}

/// Scan file content line by line for secret patterns
fn check_content(path: &Path, project_name: &str, project_path: &Path) -> Vec<SecretFinding> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut findings = Vec::new();
    let ctx = detect_context(path);
    let is_rust = path.extension().and_then(|e| e.to_str()) == Some("rs");
    let mut in_test_block = false;

    // In test/doc context, downgrade severity
    let adjust_severity = |base: Severity| -> Severity {
        match ctx {
            FindingContext::Test | FindingContext::Documentation => Severity::Info,
            FindingContext::Config => if base == Severity::Critical { Severity::Warning } else { base },
            _ => base,
        }
    };

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if is_rust && trimmed.contains("#[cfg(test)]") { in_test_block = true; }
        if in_test_block { continue; }

        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") { continue; }
        if is_test_code(trimmed) { continue; }
        if common::is_likely_false_positive(trimmed) { continue; }

        // API key patterns
        for (pattern, desc) in API_KEY_PATTERNS.iter() {
            if let Some(m) = pattern.find(trimmed) {
                findings.push(SecretFinding {
                    file_path: path.to_path_buf(), line_number: line_num + 1,
                    category: FindingCategory::ApiKey, severity: adjust_severity(Severity::Critical),
                    context: ctx.clone(), description: desc.to_string(),
                    redacted_match: common::redact(m.as_str()),
                    project_name: project_name.to_string(), project_path: project_path.to_path_buf(),
                });
            }
        }

        // Private key content — skip if it's inside quotes (string literal / regex pattern)
        if PRIVATE_KEY_CONTENT.is_match(trimmed) && !is_key_reference(trimmed) {
            findings.push(SecretFinding {
                file_path: path.to_path_buf(), line_number: line_num + 1,
                category: FindingCategory::PrivateKey, severity: adjust_severity(Severity::Critical),
                context: ctx.clone(), description: "Private Key content".into(),
                redacted_match: "-----BEGIN PRIVATE KEY-----".into(),
                project_name: project_name.to_string(), project_path: project_path.to_path_buf(),
            });
        }

        // URLs with embedded passwords (skip safe domains like Google Fonts)
        if let Some(m) = URL_WITH_PASS.find(trimmed) {
            if !is_safe_url(m.as_str()) {
                findings.push(SecretFinding {
                    file_path: path.to_path_buf(), line_number: line_num + 1,
                    category: FindingCategory::EmbeddedPassword, severity: adjust_severity(Severity::Critical),
                    context: ctx.clone(), description: "URL with embedded credentials".into(),
                    redacted_match: redact_url(m.as_str()),
                    project_name: project_name.to_string(), project_path: project_path.to_path_buf(),
                });
            }
        }

        // Generic secret assignments
        if GENERIC_SECRET.is_match(trimmed) {
            findings.push(SecretFinding {
                file_path: path.to_path_buf(), line_number: line_num + 1,
                category: FindingCategory::Credential, severity: adjust_severity(Severity::Warning),
                context: ctx.clone(), description: "Hardcoded credential assignment".into(),
                redacted_match: redact_generic(trimmed),
                project_name: project_name.to_string(), project_path: project_path.to_path_buf(),
            });
        }
    }

    findings
}

/// Check if a private key header is just a reference (inside quotes, regex, or comment)
fn is_key_reference(line: &str) -> bool {
    // Inside string literal: r"-----BEGIN", "-----BEGIN", '-----BEGIN'
    let trimmed = line.trim();
    trimmed.contains("r\"-----") || trimmed.contains("r#\"-----")
        || trimmed.contains("\"-----BEGIN") || trimmed.contains("'-----BEGIN")
        || trimmed.contains("Regex::new") || trimmed.contains("regex!")
        || trimmed.contains("contains(") || trimmed.contains("is_match")
        || trimmed.contains("static ") || trimmed.contains("const ")
}

/// Check if a line is test/fixture code writing fake secrets
fn is_test_code(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("fs::write") || lower.contains("assert!")
        || lower.contains("assert_eq!") || lower.contains("assert_ne!")
        || lower.contains("expect(") || lower.contains(".to_contain(")
        || lower.contains("mock") || lower.contains("fixture")
        || lower.contains("fn test_")
}

// is_likely_false_positive, safe_truncate, redact are in scanner::common (re-exported above)

/// Redact a URL with credentials
fn redact_url(url: &str) -> String {
    if let Some(at_pos) = url.find('@') {
        if let Some(scheme_end) = url.find("://") {
            let scheme = &url[..scheme_end + 3];
            let host = &url[at_pos..];
            return format!("{}****:****{}", scheme, host);
        }
    }
    common::redact(url)
}

/// Redact a generic key=value line
fn redact_generic(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.len() > 60 {
        format!("{}...", common::safe_truncate(trimmed, 57))
    } else {
        trimmed.to_string()
    }
}

/// Group findings by project
fn group_by_project(findings: Vec<SecretFinding>) -> Vec<AuditResult> {
    use std::collections::BTreeMap;

    let mut groups: BTreeMap<PathBuf, (String, Vec<SecretFinding>)> = BTreeMap::new();
    for f in findings {
        let entry = groups.entry(f.project_path.clone())
            .or_insert_with(|| (f.project_name.clone(), Vec::new()));
        entry.1.push(f);
    }

    let mut results: Vec<AuditResult> = groups.into_iter().map(|(path, (name, mut findings))| {
        findings.sort_by(|a, b| a.context.cmp(&b.context)
            .then(a.severity.cmp(&b.severity))
            .then(a.file_path.cmp(&b.file_path))
            .then(a.line_number.cmp(&b.line_number)));
        let critical = findings.iter().filter(|f| f.severity == Severity::Critical).count();
        let warning = findings.iter().filter(|f| f.severity == Severity::Warning).count();
        let info = findings.iter().filter(|f| f.severity == Severity::Info).count();
        AuditResult {
            project_name: name,
            project_path: path,
            findings,
            critical_count: critical,
            warning_count: warning,
            info_count: info,
        }
    }).collect();

    // Sort: most critical first
    results.sort_by(|a, b| b.critical_count.cmp(&a.critical_count)
        .then(b.warning_count.cmp(&a.warning_count)));
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
        // Init as git repo for project detection
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(all_findings.iter().any(|f| f.description.contains("AWS")));
    }

    #[test]
    fn test_github_token_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("script.sh");
        fs::write(&file, "GITHUB_TOKEN=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghij\n").unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(all_findings.iter().any(|f| f.description.contains("GitHub")));
    }

    #[test]
    fn test_env_file_is_info() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(".env");
        fs::write(&file, "PORT=3000\n").unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(all_findings.iter().any(|f| f.severity == Severity::Info && f.category == FindingCategory::EnvFile));
    }

    #[test]
    fn test_env_example_not_flagged() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join(".env.example");
        fs::write(&file, "API_KEY=your_key_here\n").unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(!all_findings.iter().any(|f| f.category == FindingCategory::EnvFile));
    }

    #[test]
    fn test_pem_file_is_critical() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("server.pem");
        fs::write(&file, "cert data").unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(all_findings.iter().any(|f| f.severity == Severity::Critical && f.category == FindingCategory::PrivateKey));
    }

    #[test]
    fn test_false_positive_filter() {
        assert!(common::is_likely_false_positive("api_key = 'your_api_key_here'"));
        assert!(common::is_likely_false_positive("password = 'changeme'"));
        assert!(common::is_likely_false_positive("# example: token = sk-xxx"));
        assert!(!common::is_likely_false_positive("AWS_SECRET=wJalrXUtnFEMI/K7MDENG/bPxRfi"));
    }

    #[test]
    fn test_redact() {
        assert_eq!(common::redact("AKIAIOSFODNN7EXAMPLE"), "AKIA****MPLE");
        assert_eq!(common::redact("short"), "shor****");
    }

    #[test]
    fn test_redact_url() {
        let url = "https://user:secret123@github.com/repo";
        let redacted = redact_url(url);
        assert!(redacted.contains("****"));
        assert!(!redacted.contains("secret123"));
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
        // Just verify the function doesn't panic
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
        fs::write(&file, "curl https://admin:supersecret@api.internal.io/data\n").unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(all_findings.iter().any(|f| f.category == FindingCategory::EmbeddedPassword));
    }

    #[test]
    fn test_generic_secret() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("config.py");
        fs::write(&file, "password = 'my_super_secret_password_123'\n").unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(all_findings.iter().any(|f| f.category == FindingCategory::Credential));
    }

    #[test]
    fn test_private_key_content() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("key.txt");
        fs::write(&file, "-----BEGIN RSA PRIVATE KEY-----\nMIIE...\n-----END RSA PRIVATE KEY-----\n").unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        let results = scan_directory(dir.path());
        let all_findings: Vec<_> = results.iter().flat_map(|r| &r.findings).collect();
        assert!(all_findings.iter().any(|f| f.category == FindingCategory::PrivateKey));
    }
}
