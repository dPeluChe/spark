//! OWASP Top 10:2025 regex patterns + the aggregate PatternDef catalog.

use super::{OwaspCategory, PatternSeverity};
use once_cell::sync::Lazy;
use regex::Regex;

pub(super) struct PatternDef {
    pub(super) regex: &'static Lazy<Regex>,
    pub(super) category: OwaspCategory,
    pub(super) severity: PatternSeverity,
    pub(super) description: &'static str,
    pub(super) suggestion: &'static str,
    /// File extensions this pattern applies to (empty = all).
    pub(super) extensions: &'static [&'static str],
}

// ── SQL injection (A03) — database APIs + string building ──
static SQL_CONCAT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)(?:execute|query|rawQuery|prepare|cursor)\s*\(\s*[`"'].*\$\{"#).unwrap()
});
static SQL_FSTRING: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?i)(?:execute|query|cursor\.\w+)\s*\(\s*f["']"#).unwrap());
static SQL_FORMAT: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)(?:execute|query|cursor\.\w+)\s*\(.*\.format\(|(?:execute|query)\s*\(.*%[sd]"#,
    )
    .unwrap()
});

// ── Command injection (A03) ──
static SHELL_TRUE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"subprocess\.\w+\(.*shell\s*=\s*True").unwrap());
static SYSTEM_EXEC: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?:child_process\.exec|os\.system|os\.popen)\s*\("#).unwrap());
static EVAL_CALL: Lazy<Regex> = Lazy::new(|| Regex::new(r"\beval\s*\(\s*\w").unwrap());

// ── XSS (A03) ──
static INNER_HTML: Lazy<Regex> = Lazy::new(|| Regex::new(r"\.innerHTML\s*=\s*\w").unwrap());
static DANGEROUS_HTML: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"dangerouslySetInnerHTML\s*=\s*\{").unwrap());
static DOCUMENT_WRITE: Lazy<Regex> = Lazy::new(|| Regex::new(r"document\.write\s*\(").unwrap());

// ── Insecure crypto (A04) ──
static WEAK_HASH: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
    r#"(?i)(?:md5|sha1|SHA1|MD5)\s*\(|MessageDigest\.getInstance\(\s*["'](?:MD5|SHA-?1)["']\)|createHash\(\s*["'](?:md5|sha1)["']\)"#
).unwrap()
});
static WEAK_CRYPTO_IMPORT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)(?:from|import)\s+.*\b(?:md5|des|rc4)\b").unwrap());

// ── Insecure deserialization (A08) ──
static PICKLE_LOAD: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:pickle|dill|jsonpickle|shelve)\.loads?\s*\(").unwrap());
static YAML_UNSAFE: Lazy<Regex> = Lazy::new(|| Regex::new(r"yaml\.load\s*\([^)]*\)").unwrap());
static UNSERIALIZE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:unserialize|Marshal\.load|Marshal\.restore)\s*\(").unwrap());

// ── Insecure config (A05) ──
static DEBUG_ON: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)(?:DEBUG|debug_mode)\s*[:=]\s*(?:true|True|1|["']true["'])"#).unwrap()
});
static CORS_WILDCARD: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
    r#"(?i)(?:Access-Control-Allow-Origin|allowed?_?origins?|cors|origin)\s*[:=]\s*['"*]?\s*\*"#
).unwrap()
});
static VERIFY_FALSE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(?:verify|ssl_verify|tls_verify|check_hostname)\s*[:=]\s*(?:false|False|0)")
        .unwrap()
});

// ── Insecure random (A04) ──
static MATH_RANDOM: Lazy<Regex> = Lazy::new(|| Regex::new(r"Math\.random\s*\(\)").unwrap());

// ── Path traversal (A01) ──
static PATH_TRAVERSAL: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)(?:readFile|writeFile|open|fopen)\s*\(.*(?:req\.|request\.|params\.|query\.)"#,
    )
    .unwrap()
});

pub(super) static PATTERNS: Lazy<Vec<PatternDef>> = Lazy::new(|| {
    vec![
        PatternDef {
            regex: &SQL_CONCAT,
            category: OwaspCategory::SqlInjection,
            severity: PatternSeverity::High,
            description: "SQL query built with string concatenation/interpolation",
            suggestion: "Use parameterized queries or prepared statements",
            extensions: &["js", "ts", "jsx", "tsx", "py", "rb", "php", "java", "go"],
        },
        PatternDef {
            regex: &SQL_FSTRING,
            category: OwaspCategory::SqlInjection,
            severity: PatternSeverity::High,
            description: "SQL query built with f-string interpolation",
            suggestion: "Use parameterized queries instead of f-strings",
            extensions: &["py"],
        },
        PatternDef {
            regex: &SQL_FORMAT,
            category: OwaspCategory::SqlInjection,
            severity: PatternSeverity::High,
            description: "SQL query built with format string (%s or .format())",
            suggestion: "Use parameterized queries with placeholders",
            extensions: &["py", "java", "go"],
        },
        PatternDef {
            regex: &SHELL_TRUE,
            category: OwaspCategory::CommandInjection,
            severity: PatternSeverity::High,
            description: "subprocess with shell=True allows command injection",
            suggestion: "Use shell=False with a list of arguments",
            extensions: &["py"],
        },
        PatternDef {
            regex: &SYSTEM_EXEC,
            category: OwaspCategory::CommandInjection,
            severity: PatternSeverity::High,
            description: "System command execution with dynamic input",
            suggestion: "Validate/sanitize input or use safe APIs",
            extensions: &["js", "ts", "py", "java", "rb", "php"],
        },
        PatternDef {
            regex: &EVAL_CALL,
            category: OwaspCategory::CommandInjection,
            severity: PatternSeverity::High,
            description: "eval() with dynamic input enables code injection",
            suggestion: "Avoid eval(); use JSON.parse() or safe alternatives",
            extensions: &["js", "ts", "jsx", "tsx", "py", "rb", "php"],
        },
        PatternDef {
            regex: &INNER_HTML,
            category: OwaspCategory::Xss,
            severity: PatternSeverity::High,
            description: "innerHTML assignment without sanitization",
            suggestion: "Use textContent or sanitize with DOMPurify",
            extensions: &["js", "ts", "jsx", "tsx"],
        },
        PatternDef {
            regex: &DANGEROUS_HTML,
            category: OwaspCategory::Xss,
            severity: PatternSeverity::Medium,
            description: "dangerouslySetInnerHTML usage",
            suggestion: "Sanitize HTML input with DOMPurify before rendering",
            extensions: &["js", "ts", "jsx", "tsx"],
        },
        PatternDef {
            regex: &DOCUMENT_WRITE,
            category: OwaspCategory::Xss,
            severity: PatternSeverity::Medium,
            description: "document.write() can enable XSS",
            suggestion: "Use DOM manipulation methods instead",
            extensions: &["js", "ts"],
        },
        PatternDef {
            regex: &WEAK_HASH,
            category: OwaspCategory::InsecureCrypto,
            severity: PatternSeverity::Medium,
            description: "Weak hash algorithm (MD5/SHA1)",
            suggestion: "Use SHA-256, SHA-3, or bcrypt/argon2 for passwords",
            extensions: &["js", "ts", "py", "java", "go", "rb", "php", "rs"],
        },
        PatternDef {
            regex: &WEAK_CRYPTO_IMPORT,
            category: OwaspCategory::InsecureCrypto,
            severity: PatternSeverity::Low,
            description: "Import of weak cryptographic algorithm",
            suggestion: "Use modern algorithms (AES-256, SHA-256, argon2)",
            extensions: &["py", "java", "go"],
        },
        PatternDef {
            regex: &PICKLE_LOAD,
            category: OwaspCategory::InsecureDeserialization,
            severity: PatternSeverity::High,
            description: "Unsafe deserialization (pickle/dill) allows code execution",
            suggestion: "Use JSON or validate input before deserializing",
            extensions: &["py"],
        },
        PatternDef {
            regex: &YAML_UNSAFE,
            category: OwaspCategory::InsecureDeserialization,
            severity: PatternSeverity::High,
            description: "yaml.load() without SafeLoader allows code execution",
            suggestion: "Use yaml.safe_load() or Loader=yaml.SafeLoader",
            extensions: &["py"],
        },
        PatternDef {
            regex: &UNSERIALIZE,
            category: OwaspCategory::InsecureDeserialization,
            severity: PatternSeverity::High,
            description: "Unsafe deserialization of untrusted data",
            suggestion: "Validate and sanitize input before deserializing",
            extensions: &["php", "rb"],
        },
        PatternDef {
            regex: &DEBUG_ON,
            category: OwaspCategory::InsecureConfig,
            severity: PatternSeverity::Medium,
            description: "Debug mode enabled (should be off in production)",
            suggestion: "Use environment variables: DEBUG=${DEBUG:-false}",
            extensions: &["py", "js", "ts", "yaml", "yml", "toml", "json", "env"],
        },
        PatternDef {
            regex: &CORS_WILDCARD,
            category: OwaspCategory::InsecureConfig,
            severity: PatternSeverity::Medium,
            description: "CORS allows all origins (wildcard *)",
            suggestion: "Restrict to specific trusted domains",
            extensions: &["js", "ts", "py", "java", "go", "yaml", "yml", "json"],
        },
        PatternDef {
            regex: &VERIFY_FALSE,
            category: OwaspCategory::InsecureConfig,
            severity: PatternSeverity::High,
            description: "SSL/TLS verification disabled",
            suggestion: "Enable certificate verification in production",
            extensions: &["py", "js", "ts", "java", "go", "rb"],
        },
        PatternDef {
            regex: &MATH_RANDOM,
            category: OwaspCategory::InsecureRandom,
            severity: PatternSeverity::Low,
            description: "Math.random() is not cryptographically secure",
            suggestion: "Use crypto.getRandomValues() or crypto.randomUUID()",
            extensions: &["js", "ts", "jsx", "tsx"],
        },
        PatternDef {
            regex: &PATH_TRAVERSAL,
            category: OwaspCategory::PathTraversal,
            severity: PatternSeverity::High,
            description: "File operation with user-controlled path",
            suggestion: "Validate path with path.resolve() and check against base directory",
            extensions: &["js", "ts", "py", "java", "go", "rb", "php"],
        },
    ]
});
