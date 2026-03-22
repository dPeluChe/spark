use std::time::Duration;
use tokio::process::Command;

use crate::core::types::*;

/// Execute an update for a tool
pub async fn update_tool(tool: &Tool) -> Result<(), String> {
    let timeout = Duration::from_secs(600); // 10 minutes

    match tool.method {
        UpdateMethod::Brew | UpdateMethod::BrewPkg => update_brew(tool, timeout).await,
        UpdateMethod::MacApp => update_mac_app(tool, timeout).await,
        UpdateMethod::NpmSys | UpdateMethod::NpmPkg | UpdateMethod::Claude => {
            update_npm(tool, timeout).await
        }
        UpdateMethod::Omz => update_omz(timeout).await,
        UpdateMethod::Toad => update_script(
            "curl -fsSL https://batrachian.ai/install | sh",
            "toad",
            timeout,
        )
        .await,
        UpdateMethod::Droid => {
            update_script("curl -fsSL https://app.factory.ai/cli | sh", "droid", timeout).await
        }
        UpdateMethod::Opencode => update_opencode(timeout).await,
        UpdateMethod::Manual => Err("Manual update required (check vendor portal)".into()),
    }
}

async fn update_brew(tool: &Tool, timeout: Duration) -> Result<(), String> {
    run_with_timeout("brew", &["upgrade", &tool.package], timeout).await
}

async fn update_mac_app(tool: &Tool, timeout: Duration) -> Result<(), String> {
    // Check if it's a brew cask
    let check = Command::new("brew")
        .args(["list", "--cask", &tool.package])
        .output()
        .await;

    if check.map(|o| o.status.success()).unwrap_or(false) {
        return run_with_timeout("brew", &["upgrade", "--cask", &tool.package], timeout).await;
    }

    Err("Manual update required (not a brew cask)".into())
}

async fn update_npm(tool: &Tool, timeout: Duration) -> Result<(), String> {
    let pkg = if tool.package.is_empty() {
        &tool.binary
    } else {
        &tool.package
    };
    let pkg_latest = format!("{}@latest", pkg);

    let result = run_with_timeout("npm", &["install", "-g", &pkg_latest], timeout).await;

    if let Err(ref e) = result {
        if e.contains("EEXIST") {
            // Retry with --force
            return run_with_timeout("npm", &["install", "-g", &pkg_latest, "--force"], timeout)
                .await;
        }
    }

    result
}

async fn update_omz(timeout: Duration) -> Result<(), String> {
    run_with_timeout("sh", &["-c", "$ZSH/tools/upgrade.sh"], timeout).await
}

async fn update_opencode(timeout: Duration) -> Result<(), String> {
    // Try built-in upgrade first
    if run_with_timeout("opencode", &["upgrade"], timeout).await.is_ok() {
        return Ok(());
    }
    // Fallback to install script
    update_script(
        "curl -fsSL https://opencode.ai/install | bash",
        "opencode",
        timeout,
    )
    .await
}

async fn update_script(script: &str, name: &str, timeout: Duration) -> Result<(), String> {
    run_with_timeout("sh", &["-c", script], timeout)
        .await
        .map_err(|e| format!("{} update failed: {}", name, e))
}

async fn run_with_timeout(cmd: &str, args: &[&str], timeout: Duration) -> Result<(), String> {
    let result = tokio::time::timeout(timeout, async {
        Command::new(cmd)
            .args(args)
            .output()
            .await
            .map_err(|e| format!("Failed to execute {}: {}", cmd, e))
    })
    .await;

    match result {
        Ok(Ok(output)) => {
            if output.status.success() {
                Ok(())
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                Err(format!("{}{}", stdout, stderr))
            }
        }
        Ok(Err(e)) => Err(e),
        Err(_) => Err(format!("{} timed out", cmd)),
    }
}
