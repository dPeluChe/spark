//! Dependency vulnerability scanner using OSV.dev API.
//!
//! Parses package.json, requirements.txt, and Cargo.toml to extract
//! dependencies, then queries the OSV.dev batch API for known vulnerabilities.

use std::path::Path;
use serde::{Deserialize, Serialize};

/// A dependency found in a project
#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub ecosystem: String,
    pub source_file: String,
}

/// A vulnerability found for a dependency
#[derive(Debug, Clone)]
pub struct DepVulnerability {
    pub id: String,
    pub summary: String,
    pub severity: String,
    pub dep_name: String,
    pub dep_version: String,
    pub ecosystem: String,
    pub fixed_version: Option<String>,
    pub source_file: String,
}

/// Result of dependency scanning
#[derive(Debug, Clone)]
pub struct DepScanResult {
    pub deps_checked: usize,
    pub vulnerabilities: Vec<DepVulnerability>,
}

// ─── OSV.dev API types ───

#[derive(Serialize)]
struct OsvBatchRequest {
    queries: Vec<OsvQuery>,
}

#[derive(Serialize)]
struct OsvQuery {
    package: OsvPackage,
    version: String,
}

#[derive(Serialize)]
struct OsvPackage {
    name: String,
    ecosystem: String,
}

#[derive(Deserialize)]
struct OsvBatchResponse {
    results: Vec<OsvQueryResult>,
}

#[derive(Deserialize)]
struct OsvQueryResult {
    vulns: Option<Vec<OsvVuln>>,
}

#[derive(Deserialize)]
struct OsvVuln {
    id: String,
    summary: Option<String>,
    affected: Option<Vec<OsvAffected>>,
    database_specific: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct OsvAffected {
    ranges: Option<Vec<OsvRange>>,
}

#[derive(Deserialize)]
struct OsvRange {
    events: Option<Vec<OsvEvent>>,
}

#[derive(Deserialize)]
struct OsvEvent {
    fixed: Option<String>,
}

// ─── Dependency parsing ───

/// Parse all dependency files in a directory
pub fn parse_dependencies(path: &Path) -> Vec<Dependency> {
    let mut deps = Vec::new();

    // package.json (npm)
    let pkg_json = path.join("package.json");
    if pkg_json.exists() {
        deps.extend(parse_package_json(&pkg_json));
    }

    // package-lock.json has more accurate versions
    let pkg_lock = path.join("package-lock.json");
    if pkg_lock.exists() && deps.iter().any(|d| d.ecosystem == "npm") {
        // Override with lock file versions where available
        let lock_deps = parse_package_lock(&pkg_lock);
        for ld in &lock_deps {
            if let Some(d) = deps.iter_mut().find(|d| d.name == ld.name && d.ecosystem == "npm") {
                d.version = ld.version.clone();
                d.source_file = ld.source_file.clone();
            }
        }
    }

    // requirements.txt (Python)
    let req_txt = path.join("requirements.txt");
    if req_txt.exists() {
        deps.extend(parse_requirements_txt(&req_txt));
    }

    // Cargo.lock (Rust) — more accurate than Cargo.toml
    let cargo_lock = path.join("Cargo.lock");
    if cargo_lock.exists() {
        deps.extend(parse_cargo_lock(&cargo_lock));
    } else {
        let cargo_toml = path.join("Cargo.toml");
        if cargo_toml.exists() {
            deps.extend(parse_cargo_toml(&cargo_toml));
        }
    }

    deps
}

fn parse_package_json(path: &Path) -> Vec<Dependency> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut deps = Vec::new();
    for key in &["dependencies", "devDependencies"] {
        if let Some(obj) = json.get(key).and_then(|v| v.as_object()) {
            for (name, version) in obj {
                let ver = version.as_str().unwrap_or("").to_string();
                // Strip semver prefixes: ^1.2.3 → 1.2.3, ~1.2.3 → 1.2.3
                let clean_ver = ver.trim_start_matches('^').trim_start_matches('~')
                    .trim_start_matches(">=").trim_start_matches('>')
                    .trim_start_matches("<=").trim_start_matches('<')
                    .to_string();
                if !clean_ver.is_empty() && clean_ver.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    deps.push(Dependency {
                        name: name.clone(),
                        version: clean_ver,
                        ecosystem: "npm".into(),
                        source_file: "package.json".into(),
                    });
                }
            }
        }
    }
    deps
}

fn parse_package_lock(path: &Path) -> Vec<Dependency> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut deps = Vec::new();
    // lockfileVersion 2/3 uses "packages"
    if let Some(packages) = json.get("packages").and_then(|v| v.as_object()) {
        for (key, val) in packages {
            if key.is_empty() || key == "" { continue; } // skip root
            let name = key.strip_prefix("node_modules/").unwrap_or(key);
            if name.contains("node_modules/") { continue; } // skip nested
            if let Some(version) = val.get("version").and_then(|v| v.as_str()) {
                deps.push(Dependency {
                    name: name.to_string(),
                    version: version.to_string(),
                    ecosystem: "npm".into(),
                    source_file: "package-lock.json".into(),
                });
            }
        }
    }
    // lockfileVersion 1 uses "dependencies"
    else if let Some(dependencies) = json.get("dependencies").and_then(|v| v.as_object()) {
        for (name, val) in dependencies {
            if let Some(version) = val.get("version").and_then(|v| v.as_str()) {
                deps.push(Dependency {
                    name: name.clone(),
                    version: version.to_string(),
                    ecosystem: "npm".into(),
                    source_file: "package-lock.json".into(),
                });
            }
        }
    }
    deps
}

fn parse_requirements_txt(path: &Path) -> Vec<Dependency> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    content.lines()
        .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#') && !l.trim().starts_with('-'))
        .filter_map(|line| {
            let line = line.trim();
            // package==1.2.3 or package>=1.2.3
            if let Some(pos) = line.find("==") {
                Some(Dependency {
                    name: line[..pos].trim().to_string(),
                    version: line[pos+2..].trim().to_string(),
                    ecosystem: "PyPI".into(),
                    source_file: "requirements.txt".into(),
                })
            } else if let Some(pos) = line.find(">=") {
                Some(Dependency {
                    name: line[..pos].trim().to_string(),
                    version: line[pos+2..].split(',').next().unwrap_or("").trim().to_string(),
                    ecosystem: "PyPI".into(),
                    source_file: "requirements.txt".into(),
                })
            } else {
                None // No version pinned
            }
        })
        .filter(|d| !d.version.is_empty())
        .collect()
}

fn parse_cargo_toml(path: &Path) -> Vec<Dependency> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let toml_val: toml::Value = match content.parse() {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    for key in &["dependencies", "dev-dependencies"] {
        if let Some(table) = toml_val.get(key).and_then(|v| v.as_table()) {
            for (name, val) in table {
                let version = match val {
                    toml::Value::String(s) => s.clone(),
                    toml::Value::Table(t) => t.get("version")
                        .and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    _ => continue,
                };
                let clean = version.trim_start_matches('^').trim_start_matches('~').to_string();
                if !clean.is_empty() {
                    deps.push(Dependency {
                        name: name.clone(), version: clean,
                        ecosystem: "crates.io".into(),
                        source_file: "Cargo.toml".into(),
                    });
                }
            }
        }
    }
    deps
}

fn parse_cargo_lock(path: &Path) -> Vec<Dependency> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let toml_val: toml::Value = match content.parse() {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut deps = Vec::new();
    if let Some(packages) = toml_val.get("package").and_then(|v| v.as_array()) {
        for pkg in packages {
            let name = pkg.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let version = pkg.get("version").and_then(|v| v.as_str()).unwrap_or("");
            if !name.is_empty() && !version.is_empty() {
                deps.push(Dependency {
                    name: name.to_string(), version: version.to_string(),
                    ecosystem: "crates.io".into(),
                    source_file: "Cargo.lock".into(),
                });
            }
        }
    }
    deps
}

// ─── OSV.dev API query ───

const OSV_BATCH_URL: &str = "https://api.osv.dev/v1/querybatch";
const BATCH_SIZE: usize = 100;

/// Query OSV.dev for vulnerabilities in the given dependencies.
pub async fn check_vulnerabilities(deps: &[Dependency]) -> DepScanResult {
    if deps.is_empty() {
        return DepScanResult { deps_checked: 0, vulnerabilities: Vec::new() };
    }

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(_) => return DepScanResult { deps_checked: deps.len(), vulnerabilities: Vec::new() },
    };

    let mut all_vulns = Vec::new();

    // Process in batches to avoid oversized requests
    for chunk in deps.chunks(BATCH_SIZE) {
        let queries: Vec<OsvQuery> = chunk.iter().map(|d| OsvQuery {
            package: OsvPackage { name: d.name.clone(), ecosystem: d.ecosystem.clone() },
            version: d.version.clone(),
        }).collect();

        let request = OsvBatchRequest { queries };

        let response = match client.post(OSV_BATCH_URL).json(&request).send().await {
            Ok(r) => r,
            Err(_) => continue,
        };

        let batch: OsvBatchResponse = match response.json().await {
            Ok(b) => b,
            Err(_) => continue,
        };

        for (i, result) in batch.results.iter().enumerate() {
            if let Some(vulns) = &result.vulns {
                let dep = &chunk[i];
                for vuln in vulns {
                    let severity = vuln.database_specific.as_ref()
                        .and_then(|d| d.get("severity"))
                        .and_then(|s| s.as_str())
                        .unwrap_or("UNKNOWN")
                        .to_string();

                    let fixed = vuln.affected.as_ref()
                        .and_then(|a| a.first())
                        .and_then(|a| a.ranges.as_ref())
                        .and_then(|r| r.first())
                        .and_then(|r| r.events.as_ref())
                        .and_then(|e| e.iter().find_map(|ev| ev.fixed.clone()));

                    all_vulns.push(DepVulnerability {
                        id: vuln.id.clone(),
                        summary: vuln.summary.clone().unwrap_or_else(|| "No description".into()),
                        severity,
                        dep_name: dep.name.clone(),
                        dep_version: dep.version.clone(),
                        ecosystem: dep.ecosystem.clone(),
                        fixed_version: fixed,
                        source_file: dep.source_file.clone(),
                    });
                }
            }
        }
    }

    // Sort: critical first
    all_vulns.sort_by(|a, b| {
        severity_order(&a.severity).cmp(&severity_order(&b.severity))
            .then(a.dep_name.cmp(&b.dep_name))
    });

    DepScanResult {
        deps_checked: deps.len(),
        vulnerabilities: all_vulns,
    }
}

fn severity_order(s: &str) -> u8 {
    match s.to_uppercase().as_str() {
        "CRITICAL" => 0,
        "HIGH" => 1,
        "MODERATE" | "MEDIUM" => 2,
        "LOW" => 3,
        _ => 4,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("package.json");
        std::fs::write(&file, r#"{"dependencies":{"lodash":"^4.17.21"},"devDependencies":{"jest":"^29.0.0"}}"#).unwrap();
        let deps = parse_package_json(&file);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "lodash");
        assert_eq!(deps[0].version, "4.17.21");
        assert_eq!(deps[0].ecosystem, "npm");
    }

    #[test]
    fn test_parse_requirements_txt() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("requirements.txt");
        std::fs::write(&file, "django==4.2.0\nrequests>=2.28.0\n# comment\nflask\n").unwrap();
        let deps = parse_requirements_txt(&file);
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name, "django");
        assert_eq!(deps[0].version, "4.2.0");
        assert_eq!(deps[1].name, "requests");
    }

    #[test]
    fn test_parse_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("Cargo.toml");
        std::fs::write(&file, r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = { version = "1", features = ["full"] }
"#).unwrap();
        let deps = parse_cargo_toml(&file);
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|d| d.name == "serde" && d.version == "1.0"));
        assert!(deps.iter().any(|d| d.name == "tokio" && d.version == "1"));
    }

    #[test]
    fn test_parse_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let deps = parse_dependencies(dir.path());
        assert!(deps.is_empty());
    }

    #[test]
    fn test_severity_order() {
        assert!(severity_order("CRITICAL") < severity_order("HIGH"));
        assert!(severity_order("HIGH") < severity_order("MEDIUM"));
        assert!(severity_order("LOW") < severity_order("UNKNOWN"));
    }
}
