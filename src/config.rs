use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SparkConfig {
    /// Directories to scan for git repos
    pub scan_directories: Vec<PathBuf>,
    /// Days since last commit to consider a repo stale
    pub stale_threshold_days: u64,
    /// Minimum artifact size (bytes) to flag as large
    pub large_artifact_threshold: u64,
    /// Use OS trash instead of permanent delete
    pub use_trash: bool,
    /// Max directory depth for scanner
    pub max_scan_depth: usize,
    /// Root directory for managed repos (ghq-style: root/host/owner/name)
    #[serde(default = "default_repos_root")]
    pub repos_root: PathBuf,
}

fn default_repos_root() -> PathBuf {
    // Try ghq root first, fall back to ~/repos
    detect_ghq_root().unwrap_or_else(|| {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~")).join("repos")
    })
}

/// Detect ghq root from `ghq root` command or git config
fn detect_ghq_root() -> Option<PathBuf> {
    // Try `ghq root` first
    if let Ok(output) = std::process::Command::new("ghq").arg("root").output() {
        if output.status.success() {
            let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !root.is_empty() {
                let path = PathBuf::from(&root);
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }
    // Fallback: read git config ghq.root
    if let Ok(output) = std::process::Command::new("git")
        .args(["config", "--global", "ghq.root"])
        .output()
    {
        if output.status.success() {
            let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !root.is_empty() {
                let path = PathBuf::from(&root);
                if path.exists() {
                    return Some(path);
                }
            }
        }
    }
    None
}

impl Default for SparkConfig {
    fn default() -> Self {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        Self {
            scan_directories: vec![
                home.join("Projects"),
                home.join("Developer"),
                home.join("Code"),
                home.join("repos"),
                home.join("src"),
                home.join("workspace"),
            ],
            stale_threshold_days: 90,
            large_artifact_threshold: 100 * 1024 * 1024, // 100MB
            use_trash: true,
            max_scan_depth: 6,
            repos_root: default_repos_root(),
        }
    }
}

impl SparkConfig {
    /// Load config from ~/.config/spark/config.toml, or return defaults
    pub fn load() -> Self {
        let config_path = dirs::config_dir()
            .map(|d| d.join("spark").join("config.toml"))
            .unwrap_or_default();

        if config_path.exists() {
            if let Ok(contents) = std::fs::read_to_string(&config_path) {
                if let Ok(config) = toml::from_str(&contents) {
                    return config;
                }
            }
        }

        Self::default()
    }

    /// Save config to disk
    #[allow(dead_code)]
    pub fn save(&self) -> color_eyre::Result<()> {
        let config_dir = dirs::config_dir()
            .map(|d| d.join("spark"))
            .ok_or_else(|| color_eyre::eyre::eyre!("Could not determine config directory"))?;

        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("config.toml");
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(config_path, contents)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SparkConfig::default();
        assert!(!config.scan_directories.is_empty());
        assert_eq!(config.stale_threshold_days, 90);
        assert_eq!(config.large_artifact_threshold, 100 * 1024 * 1024);
        assert!(config.use_trash);
        assert_eq!(config.max_scan_depth, 6);
    }

    #[test]
    fn test_default_repos_root_is_valid_dir() {
        let config = SparkConfig::default();
        // Should resolve to ghq root or ~/repos
        assert!(config.repos_root.is_absolute() || config.repos_root.starts_with("~"));
    }

    #[test]
    fn test_load_returns_defaults_when_no_file() {
        let config = SparkConfig::load();
        assert_eq!(config.stale_threshold_days, 90);
    }

    #[test]
    fn test_config_deserialize() {
        let toml_str = r#"
            scan_directories = ["/tmp/test"]
            stale_threshold_days = 30
            large_artifact_threshold = 50000000
            use_trash = false
            max_scan_depth = 2
            repos_root = "/tmp/repos"
        "#;
        let config: SparkConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.stale_threshold_days, 30);
        assert!(!config.use_trash);
        assert_eq!(config.max_scan_depth, 2);
        assert_eq!(config.repos_root, PathBuf::from("/tmp/repos"));
    }
}
