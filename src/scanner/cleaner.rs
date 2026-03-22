use std::path::PathBuf;

/// What cleanup action to perform
#[derive(Debug, Clone)]
pub enum CleanAction {
    /// Remove specific artifact directories (node_modules, venvs, etc.)
    DeleteArtifacts(Vec<PathBuf>),
    /// Move repo to OS trash (reversible)
    TrashRepo(PathBuf),
    /// Permanently delete repo
    DeleteRepo(PathBuf),
}

/// Result of a cleanup operation
#[derive(Debug, Clone)]
pub struct CleanResult {
    pub action_desc: String,
    pub bytes_recovered: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// Execute a cleanup action
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
                action_desc: format!("Deleted {} artifact(s)", paths.len()),
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
            // Use trash crate or fallback to move
            if use_trash {
                // For now, move to a spark trash directory
                let trash_dir = dirs::data_dir()
                    .unwrap_or_else(|| PathBuf::from("/tmp"))
                    .join("spark-trash");
                let _ = std::fs::create_dir_all(&trash_dir);

                let dest = trash_dir.join(
                    path.file_name().unwrap_or_default(),
                );

                match std::fs::rename(path, &dest) {
                    Ok(_) => CleanResult {
                        action_desc: format!("Moved to trash: {}", path.display()),
                        bytes_recovered: size,
                        success: true,
                        error: None,
                    },
                    Err(e) => CleanResult {
                        action_desc: format!("Failed to trash: {}", path.display()),
                        bytes_recovered: 0,
                        success: false,
                        error: Some(e.to_string()),
                    },
                }
            } else {
                match std::fs::remove_dir_all(path) {
                    Ok(_) => CleanResult {
                        action_desc: format!("Deleted: {}", path.display()),
                        bytes_recovered: size,
                        success: true,
                        error: None,
                    },
                    Err(e) => CleanResult {
                        action_desc: format!("Failed to delete: {}", path.display()),
                        bytes_recovered: 0,
                        success: false,
                        error: Some(e.to_string()),
                    },
                }
            }
        }
        CleanAction::DeleteRepo(path) => {
            let size = crate::utils::fs::dir_size(path);
            match std::fs::remove_dir_all(path) {
                Ok(_) => CleanResult {
                    action_desc: format!("Permanently deleted: {}", path.display()),
                    bytes_recovered: size,
                    success: true,
                    error: None,
                },
                Err(e) => CleanResult {
                    action_desc: format!("Failed to delete: {}", path.display()),
                    bytes_recovered: 0,
                    success: false,
                    error: Some(e.to_string()),
                },
            }
        }
    }
}
