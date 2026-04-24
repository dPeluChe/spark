//! OWASP code pattern scanner: detect common security anti-patterns.
//!
//! Regex-based detection of SQL injection, command injection, XSS,
//! insecure crypto, deserialization, and other OWASP Top 10 patterns.
//!
//! Pattern definitions live in `patterns.rs`; this module owns the types
//! and the directory walker that applies them.

mod patterns;

use super::common;
use patterns::PATTERNS;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternSeverity {
    High,
    Medium,
    Low,
}

impl std::fmt::Display for PatternSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PatternSeverity::High => write!(f, "HIGH"),
            PatternSeverity::Medium => write!(f, "MEDIUM"),
            PatternSeverity::Low => write!(f, "LOW"),
        }
    }
}

/// OWASP Top 10:2025 category mapping. See https://owasp.org/Top10/
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwaspCategory {
    SqlInjection,            // A03:2025 Injection
    CommandInjection,        // A03:2025 Injection
    Xss,                     // A03:2025 Injection
    InsecureCrypto,          // A04:2025 Cryptographic Failures
    InsecureDeserialization, // A08:2025 Software & Data Integrity
    InsecureConfig,          // A05:2025 Security Misconfiguration
    InsecureRandom,          // A04:2025 Cryptographic Failures
    PathTraversal,           // A01:2025 Broken Access Control
}

impl std::fmt::Display for OwaspCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwaspCategory::SqlInjection => write!(f, "A03 Injection: SQL"),
            OwaspCategory::CommandInjection => write!(f, "A03 Injection: Command"),
            OwaspCategory::Xss => write!(f, "A03 Injection: XSS"),
            OwaspCategory::InsecureCrypto => write!(f, "A04 Crypto Failures"),
            OwaspCategory::InsecureDeserialization => write!(f, "A08 Data Integrity"),
            OwaspCategory::InsecureConfig => write!(f, "A05 Security Misconfig"),
            OwaspCategory::InsecureRandom => write!(f, "A04 Crypto Failures"),
            OwaspCategory::PathTraversal => write!(f, "A01 Broken Access Control"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PatternFinding {
    pub file_path: std::path::PathBuf,
    pub line_number: usize,
    pub category: OwaspCategory,
    pub severity: PatternSeverity,
    pub description: String,
    pub matched_text: String,
    pub suggestion: String,
}

fn is_safe_pattern(line: &str) -> bool {
    line.contains("SafeLoader")
        || line.contains("safe_load")
        || line.contains("JSON.parse")
        || line.contains("innerHTML = ''")
        || line.contains(r#"innerHTML = """#)
}

const CODE_EXT: &[&str] = &[
    "js", "ts", "jsx", "tsx", "py", "rb", "php", "java", "go", "rs", "yaml", "yml", "toml", "json",
    "env",
];

const MAX_FILE_SIZE: u64 = 512_000; // 512 KB

pub fn scan_code_patterns(path: &Path) -> Vec<PatternFinding> {
    let mut findings = Vec::new();
    let ignore_patterns = common::load_ignore_patterns(path);

    for entry in walkdir::WalkDir::new(path)
        .max_depth(8)
        .into_iter()
        .filter_entry(|e| {
            if !e.file_type().is_dir() {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            !common::SKIP_DIRS.contains(&name.as_ref())
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let file_path = entry.path();

        if !ignore_patterns.is_empty() {
            let rel = file_path
                .strip_prefix(path)
                .unwrap_or(file_path)
                .display()
                .to_string();
            if ignore_patterns
                .iter()
                .any(|p| rel.starts_with(p) || rel.contains(p))
            {
                continue;
            }
        }

        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !CODE_EXT.contains(&ext) {
            continue;
        }

        if let Ok(meta) = std::fs::metadata(file_path) {
            if meta.len() > MAX_FILE_SIZE {
                continue;
            }
        }

        let (is_test, is_docs, is_infra) = classify_file(file_path);
        let downgrade = is_test || is_docs || is_infra;

        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Skip everything after #[cfg(test)] in Rust
        let test_block_start = if ext == "rs" {
            content
                .find("#[cfg(test)]")
                .map(|pos| content[..pos].lines().count())
        } else {
            None
        };

        for (line_num, line) in content.lines().enumerate() {
            if let Some(start) = test_block_start {
                if line_num >= start {
                    break;
                }
            }

            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }

            for pattern_def in PATTERNS.iter() {
                if !pattern_def.extensions.is_empty() && !pattern_def.extensions.contains(&ext) {
                    continue;
                }

                if pattern_def.regex.is_match(trimmed) && !is_safe_pattern(trimmed) {
                    let severity = if downgrade {
                        PatternSeverity::Low
                    } else {
                        pattern_def.severity
                    };

                    let matched_text = if trimmed.len() > 80 {
                        format!("{}...", common::safe_truncate(trimmed, 77))
                    } else {
                        trimmed.to_string()
                    };

                    findings.push(PatternFinding {
                        file_path: file_path.to_path_buf(),
                        line_number: line_num + 1,
                        category: pattern_def.category.clone(),
                        severity,
                        description: pattern_def.description.to_string(),
                        matched_text,
                        suggestion: pattern_def.suggestion.to_string(),
                    });
                }
            }
        }
    }

    findings.sort_by(|a, b| {
        (a.severity as u8)
            .cmp(&(b.severity as u8))
            .then(a.file_path.cmp(&b.file_path))
            .then(a.line_number.cmp(&b.line_number))
    });
    findings
}

/// Classify a file by context so we can downgrade findings in tests/docs/scripts.
fn classify_file(file_path: &Path) -> (bool, bool, bool) {
    let path_lower = file_path.display().to_string().to_lowercase();
    let is_test = path_lower.contains("/test")
        || path_lower.contains(".test.")
        || path_lower.contains(".spec.")
        || path_lower.contains("/spec/")
        || path_lower.contains("_test.")
        || path_lower.contains("/fixtures/");
    let is_docs = path_lower.contains("/docs/")
        || path_lower.contains("/archived/")
        || path_lower.contains("/documentation/")
        || path_lower.contains("/examples/");
    let file_name_lower = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    let is_infra = file_name_lower.starts_with("install")
        || file_name_lower.starts_with("setup")
        || file_name_lower.starts_with("postinstall")
        || file_name_lower == "makefile"
        || file_name_lower == "dockerfile"
        || path_lower.contains("/scripts/ci/");
    (is_test, is_docs, is_infra)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_sql_injection_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("app.py");
        fs::write(
            &file,
            "cursor.execute(f\"SELECT * FROM users WHERE id = {user_id}\")\n",
        )
        .unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings
            .iter()
            .any(|f| f.category == OwaspCategory::SqlInjection));
    }

    #[test]
    fn test_eval_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("handler.js");
        fs::write(&file, "const result = eval(userInput);\n").unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings
            .iter()
            .any(|f| f.category == OwaspCategory::CommandInjection));
    }

    #[test]
    fn test_innerhtml_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("render.js");
        fs::write(&file, "element.innerHTML = userContent;\n").unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings.iter().any(|f| f.category == OwaspCategory::Xss));
    }

    #[test]
    fn test_weak_hash_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("auth.py");
        fs::write(
            &file,
            "import hashlib\nhash = hashlib.md5(password.encode()).hexdigest()\n",
        )
        .unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings
            .iter()
            .any(|f| f.category == OwaspCategory::InsecureCrypto));
    }

    #[test]
    fn test_pickle_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("loader.py");
        fs::write(&file, "data = pickle.loads(user_data)\n").unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings
            .iter()
            .any(|f| f.category == OwaspCategory::InsecureDeserialization));
    }

    #[test]
    fn test_debug_mode_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.py");
        fs::write(&file, "DEBUG = True\n").unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings
            .iter()
            .any(|f| f.category == OwaspCategory::InsecureConfig));
    }

    #[test]
    fn test_cors_wildcard_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("server.js");
        fs::write(&file, "app.use(cors({ origin: '*' }));\n").unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings
            .iter()
            .any(|f| f.category == OwaspCategory::InsecureConfig));
    }

    #[test]
    fn test_ssl_verify_false_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("client.py");
        fs::write(&file, "requests.get(url, verify=False)\n").unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings
            .iter()
            .any(|f| f.category == OwaspCategory::InsecureConfig));
    }

    #[test]
    fn test_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings.is_empty());
    }

    #[test]
    fn test_shell_true_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("deploy.py");
        fs::write(&file, "subprocess.call(cmd, shell=True)\n").unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings
            .iter()
            .any(|f| f.category == OwaspCategory::CommandInjection));
    }
}
