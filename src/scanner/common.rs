//! Shared constants, utilities, and types used across scanner modules.

use std::path::Path;

pub const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "vendor",
    ".next",
    "target",
    "dist",
    "build",
    "__pycache__",
    ".venv",
    "venv",
    ".tox",
    ".eggs",
    ".mypy_cache",
    ".pytest_cache",
    ".cargo",
    "Pods",
    ".gradle",
    ".claude",
    ".cursor",
    ".agents",
    ".gemini",
    ".copilot",
    ".vscode",
    ".idea",
    ".fleet",
    "dist-web",
    "dist-server",
    "dist-electron",
    ".output",
    ".nuxt",
    ".vercel",
    ".parcel-cache",
    ".turbo",
    "out",
    "coverage",
];

pub const BINARY_EXT: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "ico", "svg", "webp", "bmp", "tiff", "mp3", "mp4", "avi", "mov",
    "mkv", "wav", "flac", "zip", "tar", "gz", "bz2", "xz", "7z", "rar", "exe", "dll", "so",
    "dylib", "o", "a", "lib", "wasm", "class", "pyc", "pyo", "ttf", "otf", "woff", "woff2", "eot",
    "pdf", "doc", "docx", "xls", "xlsx", "lock", "sum",
];

pub const SAFE_URL_DOMAINS: &[&str] = &[
    "fonts.googleapis.com",
    "fonts.google.com",
    "cdn.jsdelivr.net",
    "unpkg.com",
    "cdnjs.cloudflare.com",
    "registry.npmjs.org",
];

pub const MAX_FILE_SIZE: u64 = 1_048_576;

pub fn safe_truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        return s;
    }
    match s.char_indices().take_while(|(i, _)| *i <= max).last() {
        Some((i, c)) => &s[..i + c.len_utf8()],
        None => &s[..0],
    }
}

pub fn redact(value: &str) -> String {
    if value.len() <= 8 {
        return format!("{}****", &value[..value.len().min(4)]);
    }
    format!("{}****{}", &value[..4], &value[value.len() - 4..])
}

pub fn is_likely_false_positive(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.contains("example")
        || lower.contains("placeholder")
        || lower.contains("your_")
        || lower.contains("your-")
        || lower.contains("<your")
        || lower.contains("xxx")
        || lower.contains("changeme")
        || lower.contains("todo")
        || lower.contains("fixme")
        || lower.contains("replace_me")
        || lower.contains("dummy")
        || lower.contains("fake")
        || lower.contains("test_key")
        || lower.contains("test-key")
        || lower.contains("sample")
}

pub fn load_ignore_patterns(scan_root: &Path) -> Vec<String> {
    let ignore_file = scan_root.join(".sparkauditignore");
    if !ignore_file.exists() {
        return Vec::new();
    }
    std::fs::read_to_string(&ignore_file)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
        .map(|l| l.trim().to_string())
        .collect()
}

pub fn is_ignored(file_path: &Path, scan_root: &Path, patterns: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let rel = file_path
        .strip_prefix(scan_root)
        .unwrap_or(file_path)
        .display()
        .to_string();
    patterns
        .iter()
        .any(|p| rel.starts_with(p) || rel.contains(p))
}

pub fn shorten_path(path: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    if !home.is_empty() && path.starts_with(&home) {
        format!("~{}", &path[home.len()..])
    } else {
        path.to_string()
    }
}
