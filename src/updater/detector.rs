use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use crate::core::types::*;
use crate::updater::version::{clean_version_string, parse_tool_version};
use crate::utils::shell::{run_command, run_command_lossy};

/// Version detector with async brew/npm cache for local and remote version lookups
pub struct Detector {
    outdated_cache: Arc<RwLock<HashMap<String, String>>>,
    has_warmed_up: Arc<std::sync::atomic::AtomicBool>,
}

impl Detector {
    pub fn new() -> Self {
        Self {
            outdated_cache: Arc::new(RwLock::new(HashMap::new())),
            has_warmed_up: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    /// Warm up brew and npm caches in parallel
    pub async fn warm_up_cache(&self) {
        if self
            .has_warmed_up
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            return;
        }

        let cache = self.outdated_cache.clone();
        let cache2 = self.outdated_cache.clone();

        let brew_handle = tokio::spawn(async move {
            fetch_brew_outdated(cache).await;
        });

        let npm_handle = tokio::spawn(async move {
            fetch_npm_outdated(cache2).await;
        });

        let _ = tokio::join!(brew_handle, npm_handle);
        self.has_warmed_up
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// Get local version for a tool
    pub async fn get_local_version(&self, tool: &Tool) -> String {
        match tool.method {
            UpdateMethod::MacApp => get_mac_app_version(&tool.binary).await,
            UpdateMethod::Omz => get_omz_version().await,
            UpdateMethod::Manual if tool.binary == "antigravity" => {
                get_antigravity_version().await
            }
            _ => get_cli_tool_version(tool).await,
        }
    }

    /// Get remote/latest version for a tool
    pub async fn get_remote_version(&self, tool: &Tool, local_version: &str) -> String {
        if local_version == "MISSING" {
            return "Unknown".into();
        }

        let cache = self.outdated_cache.read().await;

        if let Some(latest) = cache.get(&tool.package) {
            return latest.clone();
        }

        if self
            .has_warmed_up
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            return local_version.to_string(); // Not in outdated list = up to date
        }

        "Checking...".into()
    }
}

async fn fetch_brew_outdated(cache: Arc<RwLock<HashMap<String, String>>>) {
    let output = run_command_lossy("brew", &["outdated", "--json=v2"], Duration::from_secs(30)).await;
    if output.is_empty() {
        return;
    }

    #[derive(serde::Deserialize)]
    struct BrewItem {
        name: String,
        current_version: String,
    }
    #[derive(serde::Deserialize)]
    struct BrewOutdated {
        #[serde(default)]
        formulae: Vec<BrewItem>,
        #[serde(default)]
        casks: Vec<BrewItem>,
    }

    if let Ok(data) = serde_json::from_str::<BrewOutdated>(&output) {
        let mut cache = cache.write().await;
        for item in data.formulae.iter().chain(data.casks.iter()) {
            cache.insert(item.name.clone(), item.current_version.clone());
        }
    }
}

async fn fetch_npm_outdated(cache: Arc<RwLock<HashMap<String, String>>>) {
    let output =
        run_command_lossy("npm", &["outdated", "-g", "--json"], Duration::from_secs(30)).await;
    if output.is_empty() {
        return;
    }

    #[derive(serde::Deserialize)]
    struct NpmItem {
        latest: Option<String>,
    }

    if let Ok(data) = serde_json::from_str::<HashMap<String, NpmItem>>(&output) {
        let mut cache = cache.write().await;
        for (pkg, info) in data {
            if let Some(latest) = info.latest {
                cache.insert(pkg, latest);
            }
        }
    }
}

async fn get_mac_app_version(binary: &str) -> String {
    let app_path = match binary {
        "iterm" => "/Applications/iTerm.app",
        "ghostty" => "/Applications/Ghostty.app",
        "warp" => "/Applications/Warp.app",
        "code" => "/Applications/Visual Studio Code.app",
        "cursor" => "/Applications/Cursor.app",
        "zed" => "/Applications/Zed.app",
        "windsurf" => "/Applications/Windsurf.app",
        "docker" => "/Applications/Docker.app",
        _ => return "MISSING".into(),
    };

    let plist = format!("{}/Contents/Info.plist", app_path);
    if !std::path::Path::new(&plist).exists() {
        return "MISSING".into();
    }

    match run_command(
        "defaults",
        &["read", &plist, "CFBundleShortVersionString"],
        Duration::from_secs(5),
    )
    .await
    {
        Ok(v) => v,
        Err(_) => "Detected".into(),
    }
}

async fn get_omz_version() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let omz_path = format!("{}/.oh-my-zsh", home);
    if !std::path::Path::new(&omz_path).exists() {
        return "MISSING".into();
    }

    let git_dir = format!("{}/.git", omz_path);
    match run_command(
        "git",
        &[
            &format!("--git-dir={}", git_dir),
            &format!("--work-tree={}", omz_path),
            "rev-parse",
            "--short",
            "HEAD",
        ],
        Duration::from_secs(5),
    )
    .await
    {
        Ok(v) if !v.is_empty() => v,
        _ => "Installed".into(),
    }
}

async fn get_antigravity_version() -> String {
    let home = std::env::var("HOME").unwrap_or_default();
    let custom_path = format!("{}/.antigravity/antigravity/bin/antigravity", home);

    if std::path::Path::new(&custom_path).exists() {
        let output = run_command_lossy(&custom_path, &["--version"], Duration::from_secs(5)).await;
        if !output.is_empty() {
            return parse_tool_version("antigravity", &output);
        }
    }

    let output = run_command_lossy("antigravity", &["--version"], Duration::from_secs(5)).await;
    if !output.is_empty() {
        parse_tool_version("antigravity", &output)
    } else {
        "MISSING".into()
    }
}

async fn get_cli_tool_version(tool: &Tool) -> String {
    // Tools without version flags — just check if binary exists
    if tool.binary == "toad" {
        let exists = std::process::Command::new("which").arg(&tool.binary)
            .output().map(|o| o.status.success()).unwrap_or(false);
        return if exists { "Installed".into() } else { "MISSING".into() };
    }

    // Tools that use non-standard version flags
    let version_args: &[&str] = match tool.binary.as_str() {
        "go" => &["version"],
        "rustup" => &["--version"],
        "gcloud" => &["version"],
        _ => &["--version"],
    };

    let output = run_command_lossy(&tool.binary, version_args, Duration::from_secs(5)).await;
    if !output.is_empty() {
        return parse_tool_version(&tool.binary, &output);
    }

    // Fallback: ~/.local/bin
    let home = std::env::var("HOME").unwrap_or_default();
    let local_bin = format!("{}/.local/bin/{}", home, tool.binary);
    if std::path::Path::new(&local_bin).exists() {
        let output = run_command_lossy(&local_bin, &["--version"], Duration::from_secs(5)).await;
        if !output.is_empty() {
            return parse_tool_version(&tool.binary, &output);
        }
    }

    // Fallback: npm global list
    if matches!(
        tool.method,
        UpdateMethod::NpmPkg | UpdateMethod::NpmSys | UpdateMethod::Claude
    ) {
        let output = run_command_lossy(
            "npm",
            &["list", "-g", "--depth=0", "--json", &tool.package],
            Duration::from_secs(10),
        )
        .await;
        if output.contains("\"version\":") {
            if let Some(ver) = output
                .split("\"version\":")
                .nth(1)
                .and_then(|s| s.split('"').nth(1))
            {
                return clean_version_string(ver);
            }
        }
    }

    // Fallback: brew list
    if matches!(tool.method, UpdateMethod::BrewPkg) {
        let output = run_command_lossy(
            "brew",
            &["list", "--versions", &tool.package],
            Duration::from_secs(10),
        )
        .await;
        if !output.is_empty() {
            let fields: Vec<&str> = output.split_whitespace().collect();
            if fields.len() >= 2 {
                return clean_version_string(fields.last().unwrap());
            }
        }
    }

    "MISSING".into()
}
