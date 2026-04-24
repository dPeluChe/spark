//! Filename-based secret detection (sensitive filenames, extensions, .env files).

use super::context::detect_context;
use super::patterns::{CREDENTIAL_CONFIG_FILES, SENSITIVE_EXTENSIONS, SENSITIVE_FILES};
use super::{FindingCategory, FindingContext, SecretFinding, Severity};
use std::path::Path;

pub(super) fn check_filename(
    path: &Path,
    project_name: &str,
    project_path: &Path,
) -> Vec<SecretFinding> {
    let mut findings = Vec::new();
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    let ctx = detect_context(path);

    for (name, severity, desc) in SENSITIVE_FILES {
        if file_name.as_ref() == *name {
            findings.push(SecretFinding {
                file_path: path.to_path_buf(),
                line_number: 0,
                category: FindingCategory::SensitiveFile,
                severity: *severity,
                context: ctx.clone(),
                description: desc.to_string(),
                redacted_match: file_name.to_string(),
                project_name: project_name.to_string(),
                project_path: project_path.to_path_buf(),
            });
        }
    }

    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        for (sensitive_ext, severity, desc) in SENSITIVE_EXTENSIONS {
            if ext.eq_ignore_ascii_case(sensitive_ext) {
                findings.push(SecretFinding {
                    file_path: path.to_path_buf(),
                    line_number: 0,
                    category: FindingCategory::PrivateKey,
                    severity: *severity,
                    context: ctx.clone(),
                    description: desc.to_string(),
                    redacted_match: file_name.to_string(),
                    project_name: project_name.to_string(),
                    project_path: project_path.to_path_buf(),
                });
            }
        }
    }

    if (file_name == ".env" || file_name.starts_with(".env."))
        && !file_name.contains("example")
        && !file_name.contains("sample")
        && !file_name.contains("template")
    {
        findings.push(SecretFinding {
            file_path: path.to_path_buf(),
            line_number: 0,
            category: FindingCategory::EnvFile,
            severity: Severity::Info,
            context: FindingContext::Config,
            description: "Environment file (may contain secrets)".into(),
            redacted_match: file_name.to_string(),
            project_name: project_name.to_string(),
            project_path: project_path.to_path_buf(),
        });
    }

    for config_file in CREDENTIAL_CONFIG_FILES {
        let config_name = Path::new(config_file)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        if file_name.as_ref() == config_name.as_ref() {
            findings.push(SecretFinding {
                file_path: path.to_path_buf(),
                line_number: 0,
                category: FindingCategory::Credential,
                severity: Severity::Warning,
                context: FindingContext::Config,
                description: format!("Config file that may contain credentials ({})", config_file),
                redacted_match: file_name.to_string(),
                project_name: project_name.to_string(),
                project_path: project_path.to_path_buf(),
            });
        }
    }

    findings
}
