use std::path::{Path, PathBuf};
use crate::utils::fs::dir_size;

/// Type of build artifact or dependency cache
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactKind {
    // JavaScript / Node
    NodeModules,
    DotNext,
    Dist,
    ParcelCache,
    // Python
    PythonVenv,
    PyCache,
    PytestCache,
    MypyCache,
    Tox,
    // Rust
    RustTarget,
    // Go
    GoBin,
    // Java / JVM
    Gradle,
    MavenTarget,
    // .NET
    DotNetObj,
    DotNetBin,
    // Ruby
    BundleVendor,
    // PHP
    PhpVendor,
    // Generic
    BuildDir,
    Coverage,
    TempDir,
}

impl std::fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactKind::NodeModules => write!(f, "node_modules"),
            ArtifactKind::DotNext => write!(f, ".next"),
            ArtifactKind::Dist => write!(f, "dist"),
            ArtifactKind::ParcelCache => write!(f, ".parcel-cache"),
            ArtifactKind::PythonVenv => write!(f, "Python venv"),
            ArtifactKind::PyCache => write!(f, "__pycache__"),
            ArtifactKind::PytestCache => write!(f, ".pytest_cache"),
            ArtifactKind::MypyCache => write!(f, ".mypy_cache"),
            ArtifactKind::Tox => write!(f, ".tox"),
            ArtifactKind::RustTarget => write!(f, "Rust target"),
            ArtifactKind::GoBin => write!(f, "Go bin"),
            ArtifactKind::Gradle => write!(f, ".gradle"),
            ArtifactKind::MavenTarget => write!(f, "Maven target"),
            ArtifactKind::DotNetObj => write!(f, ".NET obj"),
            ArtifactKind::DotNetBin => write!(f, ".NET bin"),
            ArtifactKind::BundleVendor => write!(f, "Ruby vendor"),
            ArtifactKind::PhpVendor => write!(f, "PHP vendor"),
            ArtifactKind::BuildDir => write!(f, "build"),
            ArtifactKind::Coverage => write!(f, "coverage"),
            ArtifactKind::TempDir => write!(f, "temp"),
        }
    }
}

/// Information about a discovered artifact
#[derive(Debug, Clone)]
pub struct ArtifactInfo {
    pub path: PathBuf,
    pub kind: ArtifactKind,
    pub size: u64,
}

/// Artifact check: (dir_name, kind, optional_validator_file)
const ARTIFACT_CHECKS: &[(&str, fn() -> ArtifactKind, Option<&str>)] = &[
    // JavaScript / Node
    ("node_modules", || ArtifactKind::NodeModules, Some("package.json")),
    (".next", || ArtifactKind::DotNext, Some("package.json")),
    ("dist", || ArtifactKind::Dist, None),
    (".parcel-cache", || ArtifactKind::ParcelCache, Some("package.json")),
    // Python
    (".venv", || ArtifactKind::PythonVenv, None),
    ("venv", || ArtifactKind::PythonVenv, None),
    ("env", || ArtifactKind::PythonVenv, None),
    ("__pycache__", || ArtifactKind::PyCache, None),
    (".pytest_cache", || ArtifactKind::PytestCache, None),
    (".mypy_cache", || ArtifactKind::MypyCache, None),
    (".tox", || ArtifactKind::Tox, Some("tox.ini")),
    // Rust
    ("target", || ArtifactKind::RustTarget, Some("Cargo.toml")),
    // Java / JVM
    (".gradle", || ArtifactKind::Gradle, None),
    ("target", || ArtifactKind::MavenTarget, Some("pom.xml")),
    // .NET
    ("obj", || ArtifactKind::DotNetObj, None),
    ("bin", || ArtifactKind::DotNetBin, None),
    // Ruby
    ("vendor", || ArtifactKind::BundleVendor, Some("Gemfile")),
    // PHP
    ("vendor", || ArtifactKind::PhpVendor, Some("composer.json")),
    // Generic
    ("build", || ArtifactKind::BuildDir, None),
    ("coverage", || ArtifactKind::Coverage, None),
    (".nyc_output", || ArtifactKind::Coverage, Some("package.json")),
    ("tmp", || ArtifactKind::TempDir, None),
];

/// Find all build artifacts in a repository
pub fn find_artifacts(repo_path: &Path) -> Vec<ArtifactInfo> {
    let mut artifacts = Vec::new();
    let mut seen_paths = std::collections::HashSet::new();

    for &(dir_name, kind_fn, validator) in ARTIFACT_CHECKS {
        let artifact_path = repo_path.join(dir_name);
        if !artifact_path.is_dir() {
            continue;
        }

        // Skip if we already found this path (e.g. target/ matched by both Rust and Maven)
        if !seen_paths.insert(artifact_path.clone()) {
            continue;
        }

        // Validate: if a validator file is specified, check it exists at repo root
        if let Some(validator_file) = validator {
            if !repo_path.join(validator_file).exists() {
                continue;
            }
        }

        let kind = kind_fn();

        // For Python venvs, verify it's actually a venv
        if kind == ArtifactKind::PythonVenv {
            if !artifact_path.join("pyvenv.cfg").exists()
                && !artifact_path.join("bin").join("activate").exists()
            {
                continue;
            }
        }

        // For .NET bin/obj, verify it's a .NET project
        if kind == ArtifactKind::DotNetObj || kind == ArtifactKind::DotNetBin {
            let has_csproj = std::fs::read_dir(repo_path)
                .map(|entries| {
                    entries
                        .filter_map(|e| e.ok())
                        .any(|e| {
                            e.path()
                                .extension()
                                .map(|ext| ext == "csproj" || ext == "fsproj")
                                .unwrap_or(false)
                        })
                })
                .unwrap_or(false);
            if !has_csproj {
                continue;
            }
        }

        let size = dir_size(&artifact_path);
        if size > 0 {
            artifacts.push(ArtifactInfo {
                path: artifact_path,
                kind,
                size,
            });
        }
    }

    // Sort by size descending
    artifacts.sort_by(|a, b| b.size.cmp(&a.size));
    artifacts
}
