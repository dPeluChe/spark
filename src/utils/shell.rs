use std::io::IsTerminal;
use std::time::Duration;
use tokio::process::Command;

const LOG_PATH: &str = "/tmp/spark.log";

/// Rewrite the current progress line in place.
///
/// Builds the whole line first (single write, so concurrent workers can't
/// interleave mid-line) and ends with erase-to-EOL so a shorter label never
/// leaves leftovers from the previous one. No-op when stderr is not a
/// terminal, keeping piped logs free of `\r`/escape noise.
pub fn progress_line(done: usize, total: usize, label: &str) {
    if !std::io::stderr().is_terminal() {
        return;
    }
    let line = format!("\r  [{}/{}] {}\x1b[K", done, total, label);
    eprint!("{}", line);
}

/// Wipe the progress line and move past it (no-op when stderr is not a terminal).
pub fn clear_progress_line() {
    if !std::io::stderr().is_terminal() {
        return;
    }
    eprintln!("\r\x1b[K");
}

/// Initialize debug log (clears previous run)
pub fn init_log() {
    let _ = std::fs::write(
        LOG_PATH,
        format!(
            "=== SPARK started at {:?} ===\n",
            std::time::SystemTime::now()
        ),
    );
}

/// Append a debug message to /tmp/spark.log
#[allow(dead_code)]
pub fn debug_log(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(LOG_PATH)
    {
        let _ = writeln!(f, "{}", msg);
    }
}

/// Run a shell command with a timeout and return stdout as string
pub async fn run_command(cmd: &str, args: &[&str], timeout: Duration) -> Result<String, String> {
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
pub async fn run_command_lossy(cmd: &str, args: &[&str], timeout: Duration) -> String {
    run_command(cmd, args, timeout).await.unwrap_or_default()
}
