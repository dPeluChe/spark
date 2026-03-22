use std::path::{Path, PathBuf};
use crate::utils::fs::dir_size;

/// Type of build artifact
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArtifactKind {
    NodeModules,
    PythonVenv,
    RustTarget,
    GoBin,
    BuildDir,
    DotNext,
    Dist,
    PyCache,
    Gradle,
    Other(String),
}

impl std::fmt::Display for ArtifactKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArtifactKind::NodeModules => write!(f, "node_modules"),
            ArtifactKind::PythonVenv => write!(f, "Python venv"),
            ArtifactKind::RustTarget => write!(f, "Rust target"),
            ArtifactKind::GoBin => write!(f, "Go bin"),
            ArtifactKind::BuildDir => write!(f, "build"),
            ArtifactKind::DotNext => write!(f, ".next"),
            ArtifactKind::Dist => write!(f, "dist"),
            ArtifactKind::PyCache => write!(f, "__pycache__"),
            ArtifactKind::Gradle => write!(f, ".gradle"),
            ArtifactKind::Other(name) => write!(f, "{}", name),
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

/// Find all build artifacts in a repository
pub fn find_artifacts(repo_path: &Path) -> Vec<ArtifactInfo> {
    let mut artifacts = Vec::new();

    let checks: Vec<(&str, ArtifactKind, Option<&str>)> = vec![
        ("node_modules", ArtifactKind::NodeModules, Some("package.json")),
        (".venv", ArtifactKind::PythonVenv, None),
        ("venv", ArtifactKind::PythonVenv, None),
        ("env", ArtifactKind::PythonVenv, None),
        ("target", ArtifactKind::RustTarget, Some("Cargo.toml")),
        ("build", ArtifactKind::BuildDir, None),
        (".next", ArtifactKind::DotNext, Some("package.json")),
        ("dist", ArtifactKind::Dist, None),
        ("__pycache__", ArtifactKind::PyCache, None),
        (".gradle", ArtifactKind::Gradle, None),
    ];

    for (dir_name, kind, validator) in checks {
        let artifact_path = repo_path.join(dir_name);
        if !artifact_path.is_dir() {
            continue;
        }

        // Validate: if a validator file is specified, check it exists at repo root
        if let Some(validator_file) = validator {
            if !repo_path.join(validator_file).exists() {
                continue;
            }
        }

        // For Python venvs, verify it's actually a venv
        if kind == ArtifactKind::PythonVenv {
            if !artifact_path.join("pyvenv.cfg").exists()
                && !artifact_path.join("bin").join("activate").exists()
            {
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
