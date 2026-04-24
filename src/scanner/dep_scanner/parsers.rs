//! Parse dependency manifests: package.json, package-lock.json,
//! requirements.txt, Cargo.toml, Cargo.lock.

use super::Dependency;
use std::path::Path;

pub(super) fn parse_package_json(path: &Path) -> Vec<Dependency> {
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
                let clean_ver = ver
                    .trim_start_matches('^')
                    .trim_start_matches('~')
                    .trim_start_matches(">=")
                    .trim_start_matches('>')
                    .trim_start_matches("<=")
                    .trim_start_matches('<')
                    .to_string();
                if !clean_ver.is_empty()
                    && clean_ver
                        .chars()
                        .next()
                        .map(|c| c.is_ascii_digit())
                        .unwrap_or(false)
                {
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

pub(super) fn parse_package_lock(path: &Path) -> Vec<Dependency> {
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
            if key.is_empty() {
                continue; // root entry
            }
            let name = key.strip_prefix("node_modules/").unwrap_or(key);
            if name.contains("node_modules/") {
                continue; // nested install
            }
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

pub(super) fn parse_requirements_txt(path: &Path) -> Vec<Dependency> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .filter(|l| {
            !l.trim().is_empty() && !l.trim().starts_with('#') && !l.trim().starts_with('-')
        })
        .filter_map(|line| {
            let line = line.trim();
            if let Some(pos) = line.find("==") {
                Some(Dependency {
                    name: line[..pos].trim().to_string(),
                    version: line[pos + 2..].trim().to_string(),
                    ecosystem: "PyPI".into(),
                    source_file: "requirements.txt".into(),
                })
            } else {
                line.find(">=").map(|pos| Dependency {
                    name: line[..pos].trim().to_string(),
                    version: line[pos + 2..]
                        .split(',')
                        .next()
                        .unwrap_or("")
                        .trim()
                        .to_string(),
                    ecosystem: "PyPI".into(),
                    source_file: "requirements.txt".into(),
                })
            }
        })
        .filter(|d| !d.version.is_empty())
        .collect()
}

pub(super) fn parse_cargo_toml(path: &Path) -> Vec<Dependency> {
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
                    toml::Value::Table(t) => t
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    _ => continue,
                };
                let clean = version
                    .trim_start_matches('^')
                    .trim_start_matches('~')
                    .to_string();
                if !clean.is_empty() {
                    deps.push(Dependency {
                        name: name.clone(),
                        version: clean,
                        ecosystem: "crates.io".into(),
                        source_file: "Cargo.toml".into(),
                    });
                }
            }
        }
    }
    deps
}

pub(super) fn parse_cargo_lock(path: &Path) -> Vec<Dependency> {
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
                    name: name.to_string(),
                    version: version.to_string(),
                    ecosystem: "crates.io".into(),
                    source_file: "Cargo.lock".into(),
                });
            }
        }
    }
    deps
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_json() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("package.json");
        std::fs::write(
            &file,
            r#"{"dependencies":{"lodash":"^4.17.21"},"devDependencies":{"jest":"^29.0.0"}}"#,
        )
        .unwrap();
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
        std::fs::write(
            &file,
            r#"[package]
name = "test"
version = "0.1.0"

[dependencies]
serde = "1.0"
tokio = { version = "1", features = ["full"] }
"#,
        )
        .unwrap();
        let deps = parse_cargo_toml(&file);
        assert_eq!(deps.len(), 2);
        assert!(deps.iter().any(|d| d.name == "serde" && d.version == "1.0"));
        assert!(deps.iter().any(|d| d.name == "tokio" && d.version == "1"));
    }
}
