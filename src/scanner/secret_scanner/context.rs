//! Classify a file by purpose (source / config / test / docs / build artifact).
//!
//! Used to adjust severity: findings in tests or docs are downgraded to info.

use super::super::common;
use super::FindingContext;
use std::path::Path;

pub(super) fn detect_context(path: &Path) -> FindingContext {
    let path_str = path.display().to_string().to_lowercase();
    let file_name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();

    if file_name.contains(".test.")
        || file_name.contains(".spec.")
        || file_name.contains("_test.")
        || file_name.contains("_spec.")
        || path_str.contains("/tests/")
        || path_str.contains("/__tests__/")
        || path_str.contains("/test/")
        || path_str.contains("/spec/")
        || file_name.starts_with("test_")
        || file_name.starts_with("test-")
        || path_str.contains("/smoke/")
        || path_str.contains("/fixtures/")
    {
        return FindingContext::Test;
    }

    if file_name.ends_with(".md")
        || file_name.ends_with(".rst")
        || file_name.ends_with(".txt")
        || file_name.ends_with(".adoc")
        || path_str.contains("/docs/")
        || path_str.contains("/documentation/")
    {
        return FindingContext::Documentation;
    }

    if path_str.contains("/dist/")
        || path_str.contains("/build/")
        || path_str.contains("/dist-")
        || path_str.contains("/.output/")
    {
        return FindingContext::BuildArtifact;
    }

    if file_name.starts_with('.')
        || file_name.ends_with(".json")
        || file_name.ends_with(".toml")
        || file_name.ends_with(".yaml")
        || file_name.ends_with(".yml")
        || file_name.ends_with(".ini")
        || file_name.ends_with(".cfg")
        || file_name.ends_with(".conf")
        || path_str.contains("/config/")
        || path_str.contains("/scripts/")
        || file_name.contains("config")
        || file_name == "seed.ts"
        || file_name == "seed.js"
    {
        return FindingContext::Config;
    }

    FindingContext::SourceCode
}

/// A URL is safe if it matches a whitelisted domain (e.g. Google Fonts).
pub(super) fn is_safe_url(url: &str) -> bool {
    let lower = url.to_lowercase();
    common::SAFE_URL_DOMAINS
        .iter()
        .any(|domain| lower.contains(domain))
}
