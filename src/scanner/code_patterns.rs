//! OWASP code pattern scanner: detect common security anti-patterns.
//!
//! Regex-based detection of SQL injection, command injection, XSS,
//! insecure crypto, deserialization, and other OWASP Top 10 patterns.

use std::path::Path;
use once_cell::sync::Lazy;
use regex::Regex;

/// Severity of a code pattern finding
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

/// OWASP Top 10:2025 category mapping
/// Ref: https://owasp.org/Top10/
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwaspCategory {
    SqlInjection,          // A03:2025 Injection
    CommandInjection,      // A03:2025 Injection
    Xss,                   // A03:2025 Injection
    InsecureCrypto,        // A04:2025 Cryptographic Failures
    InsecureDeserialization, // A08:2025 Software & Data Integrity
    InsecureConfig,        // A05:2025 Security Misconfiguration
    InsecureRandom,        // A04:2025 Cryptographic Failures
    PathTraversal,         // A01:2025 Broken Access Control
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

/// A code pattern finding
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

// ─── Pattern definitions ───

struct PatternDef {
    regex: &'static Lazy<Regex>,
    category: OwaspCategory,
    severity: PatternSeverity,
    description: &'static str,
    suggestion: &'static str,
    /// File extensions this pattern applies to (empty = all)
    extensions: &'static [&'static str],
}

// SQL Injection — require database context, not just %s in any string
static SQL_CONCAT: Lazy<Regex> = Lazy::new(|| Regex::new(
    r#"(?i)(?:execute|query|rawQuery|prepare|cursor)\s*\(\s*[`"'].*\$\{"#
).unwrap());
static SQL_FSTRING: Lazy<Regex> = Lazy::new(|| Regex::new(
    r#"(?i)(?:execute|query|cursor\.\w+)\s*\(\s*f["']"#
).unwrap());
static SQL_FORMAT: Lazy<Regex> = Lazy::new(|| Regex::new(
    r#"(?i)(?:execute|query|cursor\.\w+)\s*\(.*\.format\(|(?:execute|query)\s*\(.*%[sd]"#
).unwrap());

// Command Injection — require actual command execution APIs
static SHELL_TRUE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"subprocess\.\w+\(.*shell\s*=\s*True"
).unwrap());
static SYSTEM_EXEC: Lazy<Regex> = Lazy::new(|| Regex::new(
    r#"(?:child_process\.exec|os\.system|os\.popen)\s*\("#
).unwrap());
static EVAL_CALL: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"\beval\s*\(\s*\w"
).unwrap());

// XSS
static INNER_HTML: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"\.innerHTML\s*=\s*\w"
).unwrap());
static DANGEROUS_HTML: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"dangerouslySetInnerHTML\s*=\s*\{"
).unwrap());
static DOCUMENT_WRITE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"document\.write\s*\("
).unwrap());

// Insecure Crypto
static WEAK_HASH: Lazy<Regex> = Lazy::new(|| Regex::new(
    r#"(?i)(?:md5|sha1|SHA1|MD5)\s*\(|MessageDigest\.getInstance\(\s*["'](?:MD5|SHA-?1)["']\)|createHash\(\s*["'](?:md5|sha1)["']\)"#
).unwrap());
static WEAK_CRYPTO_IMPORT: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?i)(?:from|import)\s+.*\b(?:md5|des|rc4)\b"
).unwrap());

// Insecure Deserialization
static PICKLE_LOAD: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?:pickle|dill|jsonpickle|shelve)\.loads?\s*\("
).unwrap());
static YAML_UNSAFE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"yaml\.load\s*\([^)]*\)"
).unwrap());
static UNSERIALIZE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?:unserialize|Marshal\.load|Marshal\.restore)\s*\("
).unwrap());

// Insecure Config
static DEBUG_ON: Lazy<Regex> = Lazy::new(|| Regex::new(
    r#"(?i)(?:DEBUG|debug_mode)\s*[:=]\s*(?:true|True|1|["']true["'])"#
).unwrap());
static CORS_WILDCARD: Lazy<Regex> = Lazy::new(|| Regex::new(
    r#"(?i)(?:Access-Control-Allow-Origin|allowed?_?origins?|cors|origin)\s*[:=]\s*['"*]?\s*\*"#
).unwrap());
static VERIFY_FALSE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?i)(?:verify|ssl_verify|tls_verify|check_hostname)\s*[:=]\s*(?:false|False|0)"
).unwrap());

// Insecure Random
static MATH_RANDOM: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"Math\.random\s*\(\)"
).unwrap());

// Path Traversal
static PATH_TRAVERSAL: Lazy<Regex> = Lazy::new(|| Regex::new(
    r#"(?i)(?:readFile|writeFile|open|fopen)\s*\(.*(?:req\.|request\.|params\.|query\.)"#
).unwrap());

static PATTERNS: Lazy<Vec<PatternDef>> = Lazy::new(|| vec![
    // SQL Injection
    PatternDef { regex: &SQL_CONCAT, category: OwaspCategory::SqlInjection, severity: PatternSeverity::High,
        description: "SQL query built with string concatenation/interpolation",
        suggestion: "Use parameterized queries or prepared statements",
        extensions: &["js", "ts", "jsx", "tsx", "py", "rb", "php", "java", "go"] },
    PatternDef { regex: &SQL_FSTRING, category: OwaspCategory::SqlInjection, severity: PatternSeverity::High,
        description: "SQL query built with f-string interpolation",
        suggestion: "Use parameterized queries instead of f-strings",
        extensions: &["py"] },
    PatternDef { regex: &SQL_FORMAT, category: OwaspCategory::SqlInjection, severity: PatternSeverity::High,
        description: "SQL query built with format string (%s or .format())",
        suggestion: "Use parameterized queries with placeholders",
        extensions: &["py", "java", "go"] },

    // Command Injection
    PatternDef { regex: &SHELL_TRUE, category: OwaspCategory::CommandInjection, severity: PatternSeverity::High,
        description: "subprocess with shell=True allows command injection",
        suggestion: "Use shell=False with a list of arguments",
        extensions: &["py"] },
    PatternDef { regex: &SYSTEM_EXEC, category: OwaspCategory::CommandInjection, severity: PatternSeverity::High,
        description: "System command execution with dynamic input",
        suggestion: "Validate/sanitize input or use safe APIs",
        extensions: &["js", "ts", "py", "java", "rb", "php"] },
    PatternDef { regex: &EVAL_CALL, category: OwaspCategory::CommandInjection, severity: PatternSeverity::High,
        description: "eval() with dynamic input enables code injection",
        suggestion: "Avoid eval(); use JSON.parse() or safe alternatives",
        extensions: &["js", "ts", "jsx", "tsx", "py", "rb", "php"] },

    // XSS
    PatternDef { regex: &INNER_HTML, category: OwaspCategory::Xss, severity: PatternSeverity::High,
        description: "innerHTML assignment without sanitization",
        suggestion: "Use textContent or sanitize with DOMPurify",
        extensions: &["js", "ts", "jsx", "tsx"] },
    PatternDef { regex: &DANGEROUS_HTML, category: OwaspCategory::Xss, severity: PatternSeverity::Medium,
        description: "dangerouslySetInnerHTML usage",
        suggestion: "Sanitize HTML input with DOMPurify before rendering",
        extensions: &["js", "ts", "jsx", "tsx"] },
    PatternDef { regex: &DOCUMENT_WRITE, category: OwaspCategory::Xss, severity: PatternSeverity::Medium,
        description: "document.write() can enable XSS",
        suggestion: "Use DOM manipulation methods instead",
        extensions: &["js", "ts"] },

    // Insecure Crypto
    PatternDef { regex: &WEAK_HASH, category: OwaspCategory::InsecureCrypto, severity: PatternSeverity::Medium,
        description: "Weak hash algorithm (MD5/SHA1)",
        suggestion: "Use SHA-256, SHA-3, or bcrypt/argon2 for passwords",
        extensions: &["js", "ts", "py", "java", "go", "rb", "php", "rs"] },
    PatternDef { regex: &WEAK_CRYPTO_IMPORT, category: OwaspCategory::InsecureCrypto, severity: PatternSeverity::Low,
        description: "Import of weak cryptographic algorithm",
        suggestion: "Use modern algorithms (AES-256, SHA-256, argon2)",
        extensions: &["py", "java", "go"] },

    // Insecure Deserialization
    PatternDef { regex: &PICKLE_LOAD, category: OwaspCategory::InsecureDeserialization, severity: PatternSeverity::High,
        description: "Unsafe deserialization (pickle/dill) allows code execution",
        suggestion: "Use JSON or validate input before deserializing",
        extensions: &["py"] },
    PatternDef { regex: &YAML_UNSAFE, category: OwaspCategory::InsecureDeserialization, severity: PatternSeverity::High,
        description: "yaml.load() without SafeLoader allows code execution",
        suggestion: "Use yaml.safe_load() or Loader=yaml.SafeLoader",
        extensions: &["py"] },
    PatternDef { regex: &UNSERIALIZE, category: OwaspCategory::InsecureDeserialization, severity: PatternSeverity::High,
        description: "Unsafe deserialization of untrusted data",
        suggestion: "Validate and sanitize input before deserializing",
        extensions: &["php", "rb"] },

    // Insecure Config
    PatternDef { regex: &DEBUG_ON, category: OwaspCategory::InsecureConfig, severity: PatternSeverity::Medium,
        description: "Debug mode enabled (should be off in production)",
        suggestion: "Use environment variables: DEBUG=${DEBUG:-false}",
        extensions: &["py", "js", "ts", "yaml", "yml", "toml", "json", "env"] },
    PatternDef { regex: &CORS_WILDCARD, category: OwaspCategory::InsecureConfig, severity: PatternSeverity::Medium,
        description: "CORS allows all origins (wildcard *)",
        suggestion: "Restrict to specific trusted domains",
        extensions: &["js", "ts", "py", "java", "go", "yaml", "yml", "json"] },
    PatternDef { regex: &VERIFY_FALSE, category: OwaspCategory::InsecureConfig, severity: PatternSeverity::High,
        description: "SSL/TLS verification disabled",
        suggestion: "Enable certificate verification in production",
        extensions: &["py", "js", "ts", "java", "go", "rb"] },

    // Insecure Random
    PatternDef { regex: &MATH_RANDOM, category: OwaspCategory::InsecureRandom, severity: PatternSeverity::Low,
        description: "Math.random() is not cryptographically secure",
        suggestion: "Use crypto.getRandomValues() or crypto.randomUUID()",
        extensions: &["js", "ts", "jsx", "tsx"] },

    // Path Traversal
    PatternDef { regex: &PATH_TRAVERSAL, category: OwaspCategory::PathTraversal, severity: PatternSeverity::High,
        description: "File operation with user-controlled path",
        suggestion: "Validate path with path.resolve() and check against base directory",
        extensions: &["js", "ts", "py", "java", "go", "rb", "php"] },
]);

/// Directories to skip
use super::secret_scanner::COMMON_SKIP_DIRS;

/// Extensions to scan for code patterns
const CODE_EXT: &[&str] = &[
    "js", "ts", "jsx", "tsx", "py", "rb", "php", "java", "go", "rs",
    "yaml", "yml", "toml", "json", "env",
];

const MAX_FILE_SIZE: u64 = 512_000; // 512KB

/// Scan a directory for OWASP code patterns.
pub fn scan_code_patterns(path: &Path) -> Vec<PatternFinding> {
    let mut findings = Vec::new();
    let ignore_patterns = super::secret_scanner::load_ignore_patterns(path);

    for entry in walkdir::WalkDir::new(path)
        .max_depth(8)
        .into_iter()
        .filter_entry(|e| {
            if !e.file_type().is_dir() { return true; }
            let name = e.file_name().to_string_lossy();
            !COMMON_SKIP_DIRS.contains(&name.as_ref())
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() { continue; }
        let file_path = entry.path();

        // Skip files matching .sparkauditignore
        if !ignore_patterns.is_empty() {
            let rel = file_path.strip_prefix(path).unwrap_or(file_path).display().to_string();
            if ignore_patterns.iter().any(|p| rel.starts_with(p) || rel.contains(p)) { continue; }
        }

        let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !CODE_EXT.contains(&ext) { continue; }

        if let Ok(meta) = std::fs::metadata(file_path) {
            if meta.len() > MAX_FILE_SIZE { continue; }
        }

        // Skip test files for lower noise
        let path_lower = file_path.display().to_string().to_lowercase();
        let is_test = path_lower.contains("/test") || path_lower.contains(".test.")
            || path_lower.contains(".spec.") || path_lower.contains("/spec/")
            || path_lower.contains("_test.") || path_lower.contains("/fixtures/");
        let is_docs = path_lower.contains("/docs/") || path_lower.contains("/archived/")
            || path_lower.contains("/documentation/") || path_lower.contains("/examples/");
        let file_name_lower = file_path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        let is_infra = file_name_lower.starts_with("install")
            || file_name_lower.starts_with("setup")
            || file_name_lower.starts_with("postinstall")
            || file_name_lower == "makefile"
            || file_name_lower == "dockerfile"
            || path_lower.contains("/scripts/ci/");

        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Check #[cfg(test)] for Rust
        let test_block_start = if ext == "rs" {
            content.find("#[cfg(test)]").map(|pos| {
                content[..pos].lines().count()
            })
        } else {
            None
        };

        for (line_num, line) in content.lines().enumerate() {
            // Skip after #[cfg(test)] in Rust
            if let Some(start) = test_block_start {
                if line_num >= start { break; }
            }

            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with('#') {
                continue;
            }

            for pattern_def in PATTERNS.iter() {
                // Check extension filter
                if !pattern_def.extensions.is_empty() && !pattern_def.extensions.contains(&ext) {
                    continue;
                }

                if pattern_def.regex.is_match(trimmed)
                    // yaml.load with SafeLoader is fine
                    && !(trimmed.contains("SafeLoader") || trimmed.contains("safe_load"))
                    // eval(JSON.parse(...)) is fine
                    && !(trimmed.contains("JSON.parse"))
                    // innerHTML = '' or innerHTML = "" (clearing) is fine
                    && !(trimmed.contains("innerHTML = ''") || trimmed.contains(r#"innerHTML = """#))
                {
                    let severity = if is_test || is_docs || is_infra { PatternSeverity::Low } else { pattern_def.severity };

                    let matched_text = if trimmed.len() > 80 {
                        format!("{}...", super::secret_scanner::safe_truncate(trimmed, 77))
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
        (a.severity as u8).cmp(&(b.severity as u8))
            .then(a.file_path.cmp(&b.file_path))
            .then(a.line_number.cmp(&b.line_number))
    });
    findings
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_sql_injection_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("app.py");
        fs::write(&file, "cursor.execute(f\"SELECT * FROM users WHERE id = {user_id}\")\n").unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings.iter().any(|f| f.category == OwaspCategory::SqlInjection));
    }

    #[test]
    fn test_eval_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("handler.js");
        fs::write(&file, "const result = eval(userInput);\n").unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings.iter().any(|f| f.category == OwaspCategory::CommandInjection));
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
        fs::write(&file, "import hashlib\nhash = hashlib.md5(password.encode()).hexdigest()\n").unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings.iter().any(|f| f.category == OwaspCategory::InsecureCrypto));
    }

    #[test]
    fn test_pickle_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("loader.py");
        fs::write(&file, "data = pickle.loads(user_data)\n").unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings.iter().any(|f| f.category == OwaspCategory::InsecureDeserialization));
    }

    #[test]
    fn test_debug_mode_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings.py");
        fs::write(&file, "DEBUG = True\n").unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings.iter().any(|f| f.category == OwaspCategory::InsecureConfig));
    }

    #[test]
    fn test_cors_wildcard_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("server.js");
        fs::write(&file, "app.use(cors({ origin: '*' }));\n").unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings.iter().any(|f| f.category == OwaspCategory::InsecureConfig));
    }

    #[test]
    fn test_ssl_verify_false_detection() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("client.py");
        fs::write(&file, "requests.get(url, verify=False)\n").unwrap();
        let findings = scan_code_patterns(dir.path());
        assert!(findings.iter().any(|f| f.category == OwaspCategory::InsecureConfig));
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
        assert!(findings.iter().any(|f| f.category == OwaspCategory::CommandInjection));
    }
}
