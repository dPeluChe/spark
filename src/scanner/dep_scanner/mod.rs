//! Dependency vulnerability scanner using OSV.dev API.
//!
//! Parses package.json, requirements.txt, Cargo.toml/lock to extract
//! dependencies, then queries the OSV.dev batch API for known vulnerabilities.

mod parsers;

use parsers::{
    parse_cargo_lock, parse_cargo_toml, parse_package_json, parse_package_lock,
    parse_requirements_txt,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub ecosystem: String,
    pub source_file: String,
}

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

const OSV_BATCH_URL: &str = "https://api.osv.dev/v1/querybatch";
const BATCH_SIZE: usize = 100;

pub fn parse_dependencies(path: &Path) -> Vec<Dependency> {
    let mut deps = Vec::new();

    let pkg_json = path.join("package.json");
    if pkg_json.exists() {
        deps.extend(parse_package_json(&pkg_json));
    }

    // package-lock.json has more accurate versions; use it to override if present
    let pkg_lock = path.join("package-lock.json");
    if pkg_lock.exists() && deps.iter().any(|d| d.ecosystem == "npm") {
        let lock_deps = parse_package_lock(&pkg_lock);
        for ld in &lock_deps {
            if let Some(d) = deps
                .iter_mut()
                .find(|d| d.name == ld.name && d.ecosystem == "npm")
            {
                d.version = ld.version.clone();
                d.source_file = ld.source_file.clone();
            }
        }
    }

    let req_txt = path.join("requirements.txt");
    if req_txt.exists() {
        deps.extend(parse_requirements_txt(&req_txt));
    }

    // Cargo.lock has more accurate versions than Cargo.toml
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

/// Query OSV.dev for vulnerabilities in the given dependencies.
pub async fn check_vulnerabilities(deps: &[Dependency]) -> DepScanResult {
    if deps.is_empty() {
        return DepScanResult {
            deps_checked: 0,
            vulnerabilities: Vec::new(),
        };
    }

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(_) => {
            return DepScanResult {
                deps_checked: deps.len(),
                vulnerabilities: Vec::new(),
            }
        }
    };

    let mut all_vulns = Vec::new();

    for chunk in deps.chunks(BATCH_SIZE) {
        let queries: Vec<OsvQuery> = chunk
            .iter()
            .map(|d| OsvQuery {
                package: OsvPackage {
                    name: d.name.clone(),
                    ecosystem: d.ecosystem.clone(),
                },
                version: d.version.clone(),
            })
            .collect();

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
                    all_vulns.push(build_vulnerability(vuln, dep));
                }
            }
        }
    }

    // Critical first
    all_vulns.sort_by(|a, b| {
        severity_order(&a.severity)
            .cmp(&severity_order(&b.severity))
            .then(a.dep_name.cmp(&b.dep_name))
    });

    DepScanResult {
        deps_checked: deps.len(),
        vulnerabilities: all_vulns,
    }
}

fn build_vulnerability(vuln: &OsvVuln, dep: &Dependency) -> DepVulnerability {
    let severity = vuln
        .database_specific
        .as_ref()
        .and_then(|d| d.get("severity"))
        .and_then(|s| s.as_str())
        .unwrap_or("UNKNOWN")
        .to_string();

    let fixed = vuln
        .affected
        .as_ref()
        .and_then(|a| a.first())
        .and_then(|a| a.ranges.as_ref())
        .and_then(|r| r.first())
        .and_then(|r| r.events.as_ref())
        .and_then(|e| e.iter().find_map(|ev| ev.fixed.clone()));

    DepVulnerability {
        id: vuln.id.clone(),
        summary: vuln
            .summary
            .clone()
            .unwrap_or_else(|| "No description".into()),
        severity,
        dep_name: dep.name.clone(),
        dep_version: dep.version.clone(),
        ecosystem: dep.ecosystem.clone(),
        fixed_version: fixed,
        source_file: dep.source_file.clone(),
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
