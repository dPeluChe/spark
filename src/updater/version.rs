use once_cell::sync::Lazy;
use regex::Regex;

// Version parsing patterns
static SEMVER: Lazy<Regex> = Lazy::new(|| Regex::new(r"v?(\d+\.\d+\.\d+[\w\-\+]*)").unwrap());
static MAJOR_MINOR: Lazy<Regex> = Lazy::new(|| Regex::new(r"v?(\d+\.\d+)").unwrap());
static DATE_VERSION: Lazy<Regex> = Lazy::new(|| Regex::new(r"(\d{4}\.\d+\.\d+)").unwrap());
static GIT_HASH: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b([a-f0-9]{7,40})\b").unwrap());
static SIMPLE_NUMBER: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b(\d+)\b").unwrap());

/// Clean and extract version from command output
pub fn clean_version_string(output: &str) -> String {
    if output.is_empty() {
        return "Unknown".into();
    }

    let first_line = output.lines().next().unwrap_or("").trim();

    // Try patterns in order of specificity
    if let Some(v) = extract_pattern(first_line, &SEMVER) {
        return clean_version(&v);
    }
    if let Some(v) = extract_pattern(first_line, &MAJOR_MINOR) {
        return clean_version(&v);
    }
    if let Some(v) = extract_pattern(first_line, &DATE_VERSION) {
        return clean_version(&v);
    }
    if let Some(v) = extract_pattern(first_line, &GIT_HASH) {
        return v[..7].to_string();
    }
    if let Some(v) = extract_pattern(first_line, &SIMPLE_NUMBER) {
        return v;
    }

    // Look for version-like words
    for word in first_line.split_whitespace() {
        if is_version_like(word) {
            return clean_version(word);
        }
    }

    // Last resort
    if first_line.len() > 30 {
        format!("{}…", &first_line[..30])
    } else {
        first_line.to_string()
    }
}

fn extract_pattern(input: &str, pattern: &Regex) -> Option<String> {
    pattern
        .captures(input)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn clean_version(version: &str) -> String {
    version
        .trim_start_matches('v')
        .trim_start_matches('V')
        .trim_end_matches(&['.', '-'][..])
        .to_string()
}

fn is_version_like(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let first = s.as_bytes()[0];
    if first.is_ascii_digit() {
        return true;
    }
    if (first == b'v' || first == b'V') && s.len() > 1 && s.as_bytes()[1].is_ascii_digit() {
        return true;
    }
    false
}

/// Parse version for specific tools that have non-standard output
pub fn parse_tool_version(binary: &str, output: &str) -> String {
    let output = output.trim();
    if output.is_empty() {
        return "Unknown".into();
    }

    match binary {
        "aws" => {
            // aws-cli/2.22.35 Python/3.11.9 Darwin/24.0.0
            if let Some(first) = output.split_whitespace().next() {
                if let Some(ver) = first.split('/').nth(1) {
                    return ver.to_string();
                }
            }
        }
        "go" => {
            // go version go1.23.4 darwin/arm64
            let parts: Vec<&str> = output.split_whitespace().collect();
            if parts.len() >= 3 {
                return parts[2].trim_start_matches("go").to_string();
            }
        }
        "python3" | "python" => {
            // Python 3.13.1
            for part in output.split_whitespace() {
                if SEMVER.is_match(part) {
                    return clean_version(part);
                }
            }
        }
        "node" => return clean_version(output),
        "npm" => return clean_version(output),
        "docker" => {
            // Docker version 24.0.7, build afdd53b
            let parts: Vec<&str> = output.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if *part == "version" {
                    if let Some(ver) = parts.get(i + 1) {
                        return clean_version(ver.trim_end_matches(','));
                    }
                }
            }
        }
        "brew" => {
            // Homebrew 4.2.0
            let parts: Vec<&str> = output.split_whitespace().collect();
            if parts.len() >= 2 {
                return clean_version(parts[1]);
            }
        }
        "git" => {
            // git version 2.43.0
            let parts: Vec<&str> = output.split_whitespace().collect();
            for (i, part) in parts.iter().enumerate() {
                if *part == "version" {
                    if let Some(ver) = parts.get(i + 1) {
                        return clean_version(ver);
                    }
                }
            }
        }
        _ => {}
    }

    clean_version_string(output)
}
