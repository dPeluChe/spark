use std::time::Duration;
use tokio::process::Command;

/// Run a shell command with a timeout and return stdout as string
pub async fn run_command(
    cmd: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<String, String> {
    let result = tokio::time::timeout(timeout, async {
        let output = Command::new(cmd)
            .args(args)
            .output()
            .await
            .map_err(|e| format!("Failed to execute {}: {}", cmd, e))?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

        if stdout.is_empty() && !stderr.is_empty() {
            Ok(stderr)
        } else {
            Ok(stdout)
        }
    })
    .await;

    match result {
        Ok(inner) => inner,
        Err(_) => Err(format!("{} timed out after {:?}", cmd, timeout)),
    }
}

/// Run a shell command and return combined output, ignoring exit code
pub async fn run_command_lossy(
    cmd: &str,
    args: &[&str],
    timeout: Duration,
) -> String {
    run_command(cmd, args, timeout).await.unwrap_or_default()
}
