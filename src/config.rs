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
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("~")).join("repos")
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
            max_scan_depth: 4,
            repos_root: home.join("repos"),
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
