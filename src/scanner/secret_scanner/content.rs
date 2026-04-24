//! Content-based secret detection: scan file lines for API keys, embedded
//! passwords in URLs, private key blocks, and generic `password = "..."` forms.

use super::super::common;
use super::context::{detect_context, is_safe_url};
use super::patterns::{API_KEY_PATTERNS, GENERIC_SECRET, PRIVATE_KEY_CONTENT, URL_WITH_PASS};
use super::{FindingCategory, FindingContext, SecretFinding, Severity};
use std::path::Path;

pub(super) fn check_content(
    path: &Path,
    project_name: &str,
    project_path: &Path,
) -> Vec<SecretFinding> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut findings = Vec::new();
    let ctx = detect_context(path);
    let is_rust = path.extension().and_then(|e| e.to_str()) == Some("rs");
    let mut in_test_block = false;

    let adjust_severity = |base: Severity| -> Severity {
        match ctx {
            FindingContext::Test | FindingContext::Documentation => Severity::Info,
            FindingContext::Config => {
                if base == Severity::Critical {
                    Severity::Warning
                } else {
                    base
                }
            }
            _ => base,
        }
    };

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        if is_rust && trimmed.contains("#[cfg(test)]") {
            in_test_block = true;
        }
        if in_test_block {
            continue;
        }

        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        if is_test_code(trimmed) {
            continue;
        }
        if common::is_likely_false_positive(trimmed) {
            continue;
        }

        for (pattern, desc) in API_KEY_PATTERNS.iter() {
            if let Some(m) = pattern.find(trimmed) {
                findings.push(SecretFinding {
                    file_path: path.to_path_buf(),
                    line_number: line_num + 1,
                    category: FindingCategory::ApiKey,
                    severity: adjust_severity(Severity::Critical),
                    context: ctx.clone(),
                    description: desc.to_string(),
                    redacted_match: common::redact(m.as_str()),
                    project_name: project_name.to_string(),
                    project_path: project_path.to_path_buf(),
                });
            }
        }

        if PRIVATE_KEY_CONTENT.is_match(trimmed) && !is_key_reference(trimmed) {
            findings.push(SecretFinding {
                file_path: path.to_path_buf(),
                line_number: line_num + 1,
                category: FindingCategory::PrivateKey,
                severity: adjust_severity(Severity::Critical),
                context: ctx.clone(),
                description: "Private Key content".into(),
                redacted_match: "-----BEGIN PRIVATE KEY-----".into(),
                project_name: project_name.to_string(),
                project_path: project_path.to_path_buf(),
            });
        }

        if let Some(m) = URL_WITH_PASS.find(trimmed) {
            if !is_safe_url(m.as_str()) {
                findings.push(SecretFinding {
                    file_path: path.to_path_buf(),
                    line_number: line_num + 1,
                    category: FindingCategory::EmbeddedPassword,
                    severity: adjust_severity(Severity::Critical),
                    context: ctx.clone(),
                    description: "URL with embedded credentials".into(),
                    redacted_match: redact_url(m.as_str()),
                    project_name: project_name.to_string(),
                    project_path: project_path.to_path_buf(),
                });
            }
        }

        if GENERIC_SECRET.is_match(trimmed) {
            findings.push(SecretFinding {
                file_path: path.to_path_buf(),
                line_number: line_num + 1,
                category: FindingCategory::Credential,
                severity: adjust_severity(Severity::Warning),
                context: ctx.clone(),
                description: "Hardcoded credential assignment".into(),
                redacted_match: redact_generic(trimmed),
                project_name: project_name.to_string(),
                project_path: project_path.to_path_buf(),
            });
        }
    }

    findings
}

/// A private key header inside a string literal, regex, or const is a reference,
/// not an actual key. Skip those.
fn is_key_reference(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.contains("r\"-----")
        || trimmed.contains("r#\"-----")
        || trimmed.contains("\"-----BEGIN")
        || trimmed.contains("'-----BEGIN")
        || trimmed.contains("Regex::new")
        || trimmed.contains("regex!")
        || trimmed.contains("contains(")
        || trimmed.contains("is_match")
        || trimmed.contains("static ")
        || trimmed.contains("const ")
}

/// Lines that obviously write fake secrets in tests/fixtures.
fn is_test_code(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("fs::write")
        || lower.contains("assert!")
        || lower.contains("assert_eq!")
        || lower.contains("assert_ne!")
        || lower.contains("expect(")
        || lower.contains(".to_contain(")
        || lower.contains("mock")
        || lower.contains("fixture")
        || lower.contains("fn test_")
}

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

fn redact_generic(line: &str) -> String {
    let trimmed = line.trim();
    if trimmed.len() > 60 {
        format!("{}...", common::safe_truncate(trimmed, 57))
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_url() {
        let url = "https://user:secret123@github.com/repo";
        let redacted = redact_url(url);
        assert!(redacted.contains("****"));
        assert!(!redacted.contains("secret123"));
    }
}
