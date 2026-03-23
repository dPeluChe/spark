use std::path::PathBuf;

/// What cleanup action to perform
#[derive(Debug, Clone)]
pub enum CleanAction {
    /// Remove specific artifact directories (node_modules, venvs, etc.)
    DeleteArtifacts(Vec<PathBuf>),
    /// Move repo to OS trash if enabled, otherwise permanently delete
    TrashRepo(PathBuf),
}

/// Result of a cleanup operation
#[derive(Debug, Clone)]
pub struct CleanResult {
    pub bytes_recovered: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// Execute a cleanup action. When `use_trash` is true, repos are moved
/// to `~/.local/share/spark-trash/` instead of being permanently deleted.
pub fn execute_clean(action: &CleanAction, use_trash: bool) -> CleanResult {
    match action {
        CleanAction::DeleteArtifacts(paths) => {
            let mut total_recovered = 0u64;
            let mut errors = Vec::new();

            for path in paths {
                let size = crate::utils::fs::dir_size(path);
                match std::fs::remove_dir_all(path) {
                    Ok(_) => total_recovered += size,
                    Err(e) => errors.push(format!("{}: {}", path.display(), e)),
                }
            }

            CleanResult {
                bytes_recovered: total_recovered,
                success: errors.is_empty(),
                error: if errors.is_empty() {
                    None
                } else {
                    Some(errors.join("; "))
                },
            }
        }
        CleanAction::TrashRepo(path) => {
            let size = crate::utils::fs::dir_size(path);

            if use_trash {
                let trash_dir = dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join("spark-trash");
                let _ = std::fs::create_dir_all(&trash_dir);
                let dest = trash_dir.join(path.file_name().unwrap_or_default());

                match std::fs::rename(path, &dest) {
                    Ok(_) => CleanResult {
                        bytes_recovered: size,
                        success: true,
                        error: None,
                    },
                    Err(e) => CleanResult {
                        bytes_recovered: 0,
                        success: false,
                        error: Some(e.to_string()),
                    },
                }
            } else {
                remove_dir(path, size)
            }
        }
    }
}

/// Remove a directory and return a CleanResult
fn remove_dir(path: &std::path::Path, size: u64) -> CleanResult {
    match std::fs::remove_dir_all(path) {
        Ok(_) => CleanResult {
            bytes_recovered: size,
            success: true,
            error: None,
        },
        Err(e) => CleanResult {
            bytes_recovered: 0,
            success: false,
            error: Some(format!("{}: {}", path.display(), e)),
        },
    }
}
