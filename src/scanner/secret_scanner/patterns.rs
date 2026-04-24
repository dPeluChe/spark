//! Regex patterns and sensitive file lists for secret detection.

use super::Severity;
use once_cell::sync::Lazy;
use regex::Regex;

static AWS_KEY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:AKIA|ASIA|ABIA|ACCA)[0-9A-Z]{16}").unwrap());
static AWS_SECRET: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)aws[_\-]?secret[_\-]?access[_\-]?key\s*[=:]\s*['"]?([A-Za-z0-9/+=]{40})['"]?"#,
    )
    .unwrap()
});
static GITHUB_TOKEN: Lazy<Regex> = Lazy::new(|| Regex::new(r"ghp_[A-Za-z0-9]{36}").unwrap());
static GITHUB_OAUTH: Lazy<Regex> = Lazy::new(|| Regex::new(r"gho_[A-Za-z0-9]{36}").unwrap());
static ANTHROPIC_KEY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"sk-ant-api03-[A-Za-z0-9\-_]{80,}").unwrap());
static OPENAI_KEY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"sk-[A-Za-z0-9]{20}T3BlbkFJ[A-Za-z0-9]{20}").unwrap());
static GOOGLE_API: Lazy<Regex> = Lazy::new(|| Regex::new(r"AIzaSy[A-Za-z0-9\-_]{33}").unwrap());
static SENDGRID_KEY: Lazy<Regex> = Lazy::new(|| Regex::new(r"SG\.[A-Za-z0-9\-_]{22,}").unwrap());
static SLACK_TOKEN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"xox[bpors]-[0-9]{10,13}-[0-9a-zA-Z\-]{10,}").unwrap());
static STRIPE_KEY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?:sk|pk)_(?:test|live)_[A-Za-z0-9]{20,}").unwrap());
static TWILIO_KEY: Lazy<Regex> = Lazy::new(|| Regex::new(r"SK[a-f0-9]{32}").unwrap());
static JWT_TOKEN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"eyJ[A-Za-z0-9\-_]+\.eyJ[A-Za-z0-9\-_]+\.[A-Za-z0-9\-_]+").unwrap());
pub(super) static URL_WITH_PASS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[a-z]+://[^:]+:[^@\s]{3,}@[^\s]+").unwrap());
pub(super) static GENERIC_SECRET: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)(?:password|passwd|secret|token|api[_\-]?key|apikey|auth[_\-]?token|access[_\-]?key)\s*[=:]\s*['"]([^'"]{8,})['"]"#).unwrap()
});
static NPM_TOKEN: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"//registry\.npmjs\.org/:_authToken=\S+").unwrap());

pub static PRIVATE_KEY_CONTENT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----").unwrap());

/// API key patterns: (regex, description) — shared with history_scanner.
pub static API_KEY_PATTERNS: Lazy<Vec<(&Regex, &str)>> = Lazy::new(|| {
    vec![
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
    ]
});

/// Sensitive filenames (exact match).
pub(super) const SENSITIVE_FILES: &[(&str, Severity, &str)] = &[
    ("id_rsa", Severity::Critical, "SSH Private Key"),
    ("id_ecdsa", Severity::Critical, "SSH Private Key (ECDSA)"),
    (
        "id_ed25519",
        Severity::Critical,
        "SSH Private Key (Ed25519)",
    ),
    ("id_dsa", Severity::Critical, "SSH Private Key (DSA)"),
    (".htpasswd", Severity::Critical, "Apache Password File"),
    ("credentials.json", Severity::Warning, "Credentials File"),
    (
        "service-account.json",
        Severity::Warning,
        "Service Account Credentials",
    ),
    ("keystore.jks", Severity::Warning, "Java Keystore"),
    (".git-credentials", Severity::Critical, "Git Credentials"),
    (".netrc", Severity::Warning, "Netrc Credentials"),
];

/// Sensitive extensions.
pub(super) const SENSITIVE_EXTENSIONS: &[(&str, Severity, &str)] = &[
    ("pem", Severity::Critical, "PEM Certificate/Key"),
    ("key", Severity::Critical, "Private Key File"),
    ("p12", Severity::Warning, "PKCS#12 Certificate"),
    ("pfx", Severity::Warning, "PFX Certificate"),
    ("jks", Severity::Warning, "Java Keystore"),
    ("keystore", Severity::Warning, "Keystore File"),
];

/// Config files that may contain credentials.
pub(super) const CREDENTIAL_CONFIG_FILES: &[&str] = &[
    ".npmrc",
    ".pypirc",
    ".docker/config.json",
    ".terraformrc",
    "credentials.tfrc.json",
];
