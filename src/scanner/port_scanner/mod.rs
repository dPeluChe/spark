//! Port scanner: detect listening TCP ports and their owning processes.
//!
//! Platform split:
//! - macos.rs — `lsof -iTCP -sTCP:LISTEN` plus batched ps/lsof for metadata
//! - linux.rs — /proc/net/tcp + /proc/<pid>/fd socket inode correlation
//! - runtime.rs — process name / cmdline → Runtime classification
//!   and cwd / cmdline → project directory resolution

mod linux;
mod macos;
mod runtime;

use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Runtime {
    Node,
    Python,
    Go,
    Ruby,
    Java,
    Rust,
    Php,
    Dotnet,
    Elixir,
    Deno,
    Bun,
    Nginx,
    Docker,
    Other(String),
}

impl std::fmt::Display for Runtime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Runtime::Node => write!(f, "Node.js"),
            Runtime::Python => write!(f, "Python"),
            Runtime::Go => write!(f, "Go"),
            Runtime::Ruby => write!(f, "Ruby"),
            Runtime::Java => write!(f, "Java"),
            Runtime::Rust => write!(f, "Rust"),
            Runtime::Php => write!(f, "PHP"),
            Runtime::Dotnet => write!(f, ".NET"),
            Runtime::Elixir => write!(f, "Elixir"),
            Runtime::Deno => write!(f, "Deno"),
            Runtime::Bun => write!(f, "Bun"),
            Runtime::Nginx => write!(f, "nginx"),
            Runtime::Docker => write!(f, "Docker"),
            Runtime::Other(name) => write!(f, "{}", name),
        }
    }
}

impl Runtime {
    pub fn short_label(&self) -> &str {
        match self {
            Runtime::Node => "JS",
            Runtime::Python => "PY",
            Runtime::Go => "GO",
            Runtime::Ruby => "RB",
            Runtime::Java => "JV",
            Runtime::Rust => "RS",
            Runtime::Php => "PHP",
            Runtime::Dotnet => "NET",
            Runtime::Elixir => "EX",
            Runtime::Deno => "DN",
            Runtime::Bun => "BN",
            Runtime::Nginx => "NGX",
            Runtime::Docker => "DKR",
            Runtime::Other(_) => "???",
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct PortInfo {
    pub port: u16,
    pub pid: u32,
    pub process_name: String,
    pub cmdline: String,
    pub cwd: Option<PathBuf>,
    pub runtime: Runtime,
    pub project_dir: Option<String>,
}

/// Common dev server ports to highlight.
const DEV_PORTS: &[u16] = &[
    3000, 3001, 3030, 3333, 4000, 4200, 4321, 5000, 5173, 5174, 5500, 6006, 8000, 8080, 8081, 8443,
    8888, 8889, 9000, 9090, 9229, 19006, 24678,
];

pub fn is_dev_port(port: u16) -> bool {
    DEV_PORTS.contains(&port) || (3000..=9999).contains(&port)
}

/// A port entry belongs to a dev server (vs. a system service or desktop app).
pub fn is_dev_server(info: &PortInfo) -> bool {
    if let Runtime::Other(ref name) = info.runtime {
        let n = name.to_lowercase();
        if matches!(
            n.as_str(),
            "macos"
                | "spotify"
                | "redis"
                | "postgresql"
                | "mysql"
                | "mongodb"
                | "figma"
                | "dropbox"
                | "superset"
        ) {
            return false;
        }
    }
    let proc = info.process_name.to_lowercase();
    if matches!(
        proc.as_str(),
        "controlce"
            | "rapportd"
            | "spotify"
            | "raycast"
            | "dropbox"
            | "figma_age"
            | "zed"
            | "ollama"
            | "superset"
            | "stable"
    ) {
        return false;
    }
    // Terminal apps (Warp binary is "stable")
    if proc.contains("warp") || proc.contains("iterm") || proc.contains("ghostty") {
        return false;
    }
    if let Some(ref cwd) = info.cwd {
        if cwd.starts_with("/opt/homebrew") {
            return false;
        }
    }
    matches!(
        info.runtime,
        Runtime::Node
            | Runtime::Python
            | Runtime::Go
            | Runtime::Rust
            | Runtime::Ruby
            | Runtime::Bun
            | Runtime::Deno
    ) || is_dev_port(info.port)
}

/// Scan for listening TCP ports on the system.
pub fn scan_ports() -> Vec<PortInfo> {
    if cfg!(target_os = "macos") {
        macos::scan_ports_lsof()
    } else {
        linux::scan_ports_proc()
    }
}

/// Kill a process: SIGTERM, wait 500ms, SIGKILL if still alive.
pub fn kill_process(pid: u32) -> Result<(), String> {
    use std::process::Stdio;
    let status = Command::new("kill")
        .arg(pid.to_string())
        .stderr(Stdio::null())
        .status()
        .map_err(|e| format!("Failed to send SIGTERM: {}", e))?;

    if status.success() {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let check = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stderr(Stdio::null())
            .status();
        if check.map(|s| s.success()).unwrap_or(false) {
            let _ = Command::new("kill")
                .args(["-9", &pid.to_string()])
                .stderr(Stdio::null())
                .status();
        }
        Ok(())
    } else {
        Err(format!("kill {} returned non-zero status", pid))
    }
}

#[cfg(test)]
mod tests {
    use super::runtime::detect_runtime;
    use super::*;

    #[test]
    fn test_is_dev_port_common() {
        assert!(is_dev_port(3000));
        assert!(is_dev_port(5173));
        assert!(is_dev_port(8080));
        assert!(is_dev_port(8000));
    }

    #[test]
    fn test_is_dev_port_range() {
        assert!(is_dev_port(4567));
        assert!(!is_dev_port(80));
        assert!(!is_dev_port(443));
        assert!(!is_dev_port(22));
    }

    #[test]
    fn test_detect_runtime_node() {
        assert_eq!(detect_runtime("node", "node server.js"), Runtime::Node);
    }

    #[test]
    fn test_detect_runtime_python() {
        assert_eq!(
            detect_runtime("python3", "python3 manage.py runserver"),
            Runtime::Python
        );
    }

    #[test]
    fn test_detect_runtime_go() {
        assert_eq!(detect_runtime("unknown", "go run main.go"), Runtime::Go);
    }

    #[test]
    fn test_detect_runtime_rust() {
        assert_eq!(detect_runtime("myapp", "cargo run"), Runtime::Rust);
    }

    #[test]
    fn test_detect_runtime_unknown() {
        let rt = detect_runtime("mystery", "mystery --serve");
        assert!(matches!(rt, Runtime::Other(_)));
    }

    #[test]
    fn test_detect_runtime_redis() {
        let rt = detect_runtime("redis-server", "redis-server *:6379");
        assert_eq!(rt, Runtime::Other("Redis".into()));
    }

    #[test]
    fn test_detect_runtime_postgres() {
        let rt = detect_runtime("postgres", "postgres -D /data");
        assert_eq!(rt, Runtime::Other("PostgreSQL".into()));
    }

    #[test]
    fn test_runtime_short_label() {
        assert_eq!(Runtime::Node.short_label(), "JS");
        assert_eq!(Runtime::Python.short_label(), "PY");
        assert_eq!(Runtime::Rust.short_label(), "RS");
    }

    #[test]
    fn test_runtime_display() {
        assert_eq!(format!("{}", Runtime::Node), "Node.js");
        assert_eq!(format!("{}", Runtime::Docker), "Docker");
    }

    #[test]
    fn test_scan_ports_returns_results() {
        let ports = scan_ports();
        let _ = ports;
    }
}
