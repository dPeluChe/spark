//! Port scanner: detect listening TCP ports and their owning processes.
//!
//! Uses `lsof` on macOS and `/proc/net/tcp` on Linux to find listening sockets,
//! then resolves PIDs, command names, and detects the runtime.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Detected runtime/language for a process
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

/// Information about a listening port
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

/// Common dev server ports to highlight
const DEV_PORTS: &[u16] = &[
    3000, 3001, 3030, 3333,
    4000, 4200, 4321,
    5000, 5173, 5174, 5500,
    6006,
    8000, 8080, 8081, 8443,
    8888, 8889,
    9000, 9090,
    9229,
    19006,
    24678,
];

pub fn is_dev_port(port: u16) -> bool {
    DEV_PORTS.contains(&port) || (3000..=9999).contains(&port)
}

/// Check if a port entry is a dev server (not a system service or desktop app)
pub fn is_dev_server(info: &PortInfo) -> bool {
    // Known system runtimes
    if let Runtime::Other(ref name) = info.runtime {
        let n = name.to_lowercase();
        if matches!(n.as_str(),
            "macos" | "spotify" | "redis" | "postgresql" | "mysql"
            | "mongodb" | "figma" | "dropbox" | "superset"
        ) {
            return false;
        }
    }
    // Known system process names
    let proc = info.process_name.to_lowercase();
    if matches!(proc.as_str(),
        "controlce" | "rapportd" | "spotify" | "raycast" | "dropbox"
        | "figma_age" | "zed" | "ollama" | "superset"
    ) {
        return false;
    }
    // Homebrew services
    if let Some(ref cwd) = info.cwd {
        if cwd.starts_with("/opt/homebrew") {
            return false;
        }
    }
    matches!(
        info.runtime,
        Runtime::Node | Runtime::Python | Runtime::Go | Runtime::Rust
        | Runtime::Ruby | Runtime::Bun | Runtime::Deno
    ) || is_dev_port(info.port)
}

/// Scan for listening TCP ports on the system.
pub fn scan_ports() -> Vec<PortInfo> {
    if cfg!(target_os = "macos") {
        scan_ports_lsof()
    } else {
        scan_ports_proc()
    }
}

/// macOS: use `lsof -iTCP -sTCP:LISTEN -P -n` to find listening ports
fn scan_ports_lsof() -> Vec<PortInfo> {
    let output = match Command::new("lsof")
        .args(["-iTCP", "-sTCP:LISTEN", "-P", "-n"])
        .output()
    {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    if !output.status.success() {
        return Vec::new();
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut results: Vec<PortInfo> = Vec::new();
    let mut seen_ports: HashMap<u16, usize> = HashMap::new();

    for line in stdout.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 9 {
            continue;
        }

        let process_name = fields[0].to_string();
        let pid: u32 = match fields[1].parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        // NAME field is last, like "127.0.0.1:3000" or "*:8080"
        let name_field = fields[8];
        let port: u16 = match name_field.rsplit(':').next().and_then(|p| p.parse().ok()) {
            Some(p) => p,
            None => continue,
        };

        // Deduplicate: keep first occurrence per port, but update if we get a better match
        if let Some(&idx) = seen_ports.get(&port) {
            // If existing entry is same PID, skip (IPv4/IPv6 duplicate)
            if results[idx].pid == pid {
                continue;
            }
        }

        // Get cmdline via ps
        let cmdline = get_cmdline_macos(pid);
        let cwd = get_cwd_macos(pid);
        let runtime = detect_runtime(&process_name, &cmdline);
        let project_dir = resolve_project_dir(&cwd, &cmdline);

        if let Some(&idx) = seen_ports.get(&port) {
            // Replace if same port different PID (shouldn't happen often)
            results[idx] = PortInfo {
                port, pid, process_name, cmdline, cwd, runtime, project_dir,
            };
        } else {
            seen_ports.insert(port, results.len());
            results.push(PortInfo {
                port, pid, process_name, cmdline, cwd, runtime, project_dir,
            });
        }
    }

    results.sort_by_key(|p| p.port);
    results
}

/// Get command line for a PID on macOS
fn get_cmdline_macos(pid: u32) -> String {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .ok();

    match output {
        Some(o) if o.status.success() => {
            let cmd = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if cmd.len() > 120 {
                format!("{}...", &cmd[..117])
            } else {
                cmd
            }
        }
        _ => String::new(),
    }
}

/// Get working directory for a PID on macOS
fn get_cwd_macos(pid: u32) -> Option<PathBuf> {
    let output = Command::new("lsof")
        .args(["-a", "-p", &pid.to_string(), "-d", "cwd", "-Fn"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if let Some(path) = line.strip_prefix('n') {
            if path.starts_with('/') {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

/// Linux: use /proc/net/tcp to find listening sockets
fn scan_ports_proc() -> Vec<PortInfo> {
    let inode_to_port = match parse_proc_net_tcp() {
        Some(map) => map,
        None => return Vec::new(),
    };

    if inode_to_port.is_empty() {
        return Vec::new();
    }

    let mut results: Vec<PortInfo> = Vec::new();
    let mut seen_ports = std::collections::HashSet::new();

    let proc_entries = match std::fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    for entry in proc_entries.filter_map(|e| e.ok()) {
        let pid: u32 = match entry.file_name().to_str().and_then(|s| s.parse().ok()) {
            Some(pid) => pid,
            None => continue,
        };

        let fd_dir = format!("/proc/{}/fd", pid);
        let fd_entries = match std::fs::read_dir(&fd_dir) {
            Ok(entries) => entries,
            Err(_) => continue,
        };

        for fd_entry in fd_entries.filter_map(|e| e.ok()) {
            let link = match std::fs::read_link(fd_entry.path()) {
                Ok(l) => l,
                Err(_) => continue,
            };

            let link_str = link.to_string_lossy();
            if !link_str.starts_with("socket:[") {
                continue;
            }

            let inode_str = &link_str[8..link_str.len() - 1];
            let inode: u64 = match inode_str.parse() {
                Ok(i) => i,
                Err(_) => continue,
            };

            if let Some(&port) = inode_to_port.get(&inode) {
                if seen_ports.contains(&port) {
                    continue;
                }
                seen_ports.insert(port);

                let process_name = read_proc_field(pid, "comm");
                let cmdline = read_proc_cmdline(pid);
                let cwd = std::fs::read_link(format!("/proc/{}/cwd", pid)).ok();
                let runtime = detect_runtime(&process_name, &cmdline);
                let project_dir = resolve_project_dir(&cwd, &cmdline);

                results.push(PortInfo {
                    port, pid, process_name, cmdline, cwd, runtime, project_dir,
                });
            }
        }
    }

    results.sort_by_key(|p| p.port);
    results
}

/// Kill a process by PID
pub fn kill_process(pid: u32) -> Result<(), String> {
    let status = Command::new("kill")
        .arg(pid.to_string())
        .status()
        .map_err(|e| format!("Failed to send SIGTERM: {}", e))?;

    if status.success() {
        std::thread::sleep(std::time::Duration::from_millis(500));
        // Check if still alive
        let check = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status();
        if check.map(|s| s.success()).unwrap_or(false) {
            let _ = Command::new("kill")
                .args(["-9", &pid.to_string()])
                .status();
        }
        Ok(())
    } else {
        Err(format!("kill {} returned non-zero status", pid))
    }
}

/// Detect the runtime/language from process name and command line
pub fn detect_runtime(process_name: &str, cmdline: &str) -> Runtime {
    let name = process_name.to_lowercase();
    let cmd = cmdline.to_lowercase();

    if name == "node" || name.starts_with("node ") || cmd.contains("node ")
        || cmd.contains("ts-node") || cmd.contains("tsx ")
        || cmd.contains("next ") || cmd.contains("vite")
        || cmd.contains("webpack") || cmd.contains("esbuild")
        || cmd.contains("npm ") || cmd.contains("npx ")
        || cmd.contains("yarn ") || cmd.contains("pnpm ")
    {
        return Runtime::Node;
    }

    if name == "bun" || cmd.contains("bun ") {
        return Runtime::Bun;
    }

    if name == "deno" || cmd.contains("deno ") {
        return Runtime::Deno;
    }

    if name.starts_with("python") || name == "uvicorn" || name == "gunicorn"
        || name == "flask" || name == "django" || name == "celery"
        || name == "jupyter" || name == "ipython"
        || cmd.contains("python") || cmd.contains("uvicorn")
        || cmd.contains("gunicorn") || cmd.contains("flask")
        || cmd.contains("manage.py") || cmd.contains("jupyter")
    {
        return Runtime::Python;
    }

    if name == "ruby" || name == "puma" || name == "unicorn" || name == "rails"
        || cmd.contains("ruby") || cmd.contains("rails ")
        || cmd.contains("puma") || cmd.contains("unicorn")
        || cmd.contains("bundle exec")
    {
        return Runtime::Ruby;
    }

    if name == "java" || name.starts_with("java ") || cmd.contains("java ")
        || cmd.contains("spring") || cmd.contains("gradle")
        || cmd.contains("mvn") || cmd.contains(".jar")
    {
        return Runtime::Java;
    }

    if name == "dotnet" || cmd.contains("dotnet ") || cmd.contains(".dll") {
        return Runtime::Dotnet;
    }

    if name == "php" || name == "php-fpm" || cmd.contains("php ")
        || cmd.contains("artisan") || cmd.contains("composer")
    {
        return Runtime::Php;
    }

    if name == "beam.smp" || name == "elixir" || cmd.contains("mix ")
        || cmd.contains("phoenix") || cmd.contains("elixir")
    {
        return Runtime::Elixir;
    }

    // Rust before Go to avoid "cargo run" matching "go run"
    if cmd.contains("cargo run") || cmd.contains("cargo watch") || cmd.contains("cargo ") {
        return Runtime::Rust;
    }

    if is_go_binary(process_name) || cmd.contains("go run") {
        return Runtime::Go;
    }

    if name == "nginx" || name.starts_with("nginx") {
        return Runtime::Nginx;
    }

    if name == "docker-proxy" || name == "containerd" || name.starts_with("docker") {
        return Runtime::Docker;
    }

    // macOS specific: try to match common app names
    if name == "controlce" || name == "controlcenter" {
        return Runtime::Other("macOS".into());
    }
    if name == "rapportd" {
        return Runtime::Other("macOS".into());
    }
    if name == "spotify" {
        return Runtime::Other("Spotify".into());
    }
    if name.contains("redis") {
        return Runtime::Other("Redis".into());
    }
    if name.contains("postgres") || name.contains("postmaster") {
        return Runtime::Other("PostgreSQL".into());
    }
    if name.contains("mysql") || name.contains("mariadbd") {
        return Runtime::Other("MySQL".into());
    }
    if name.contains("mongo") {
        return Runtime::Other("MongoDB".into());
    }
    if name.contains("figma") {
        return Runtime::Other("Figma".into());
    }

    Runtime::Other(process_name.to_string())
}

fn is_go_binary(process_name: &str) -> bool {
    let go_hints = ["air", "gin", "fiber", "echo", "mux"];
    let name_lower = process_name.to_lowercase();
    go_hints.iter().any(|h| name_lower == *h)
}

fn resolve_project_dir(cwd: &Option<PathBuf>, cmdline: &str) -> Option<String> {
    if let Some(dir) = cwd {
        let mut check = dir.clone();
        loop {
            if check.join(".git").exists() {
                let name = check
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let path = check.display().to_string();
                let home = std::env::var("HOME").unwrap_or_default();
                let display = if path.starts_with(&home) {
                    format!("~{}", &path[home.len()..])
                } else {
                    path
                };
                return Some(format!("{} ({})", name, display));
            }
            if !check.pop() {
                break;
            }
        }

        let path = dir.display().to_string();
        let home = std::env::var("HOME").unwrap_or_default();
        if path.starts_with(&home) {
            return Some(format!("~{}", &path[home.len()..]));
        }
        return Some(path);
    }

    for part in cmdline.split_whitespace() {
        if part.starts_with('/') || part.starts_with("./") {
            return Some(part.to_string());
        }
    }

    None
}

// --- Linux-only helpers ---

fn parse_proc_net_tcp() -> Option<HashMap<u64, u16>> {
    let content = std::fs::read_to_string("/proc/net/tcp").ok()?;
    let mut map = HashMap::new();

    for line in content.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 10 || fields[3] != "0A" {
            continue;
        }
        let local_addr = fields[1];
        let port_hex = match local_addr.split(':').nth(1) {
            Some(h) => h,
            None => continue,
        };
        let port = match u16::from_str_radix(port_hex, 16) {
            Ok(p) => p,
            Err(_) => continue,
        };
        let inode: u64 = match fields[9].parse() {
            Ok(i) => i,
            Err(_) => continue,
        };
        map.insert(inode, port);
    }

    if let Ok(content6) = std::fs::read_to_string("/proc/net/tcp6") {
        for line in content6.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 10 || fields[3] != "0A" {
                continue;
            }
            if let Some(port_hex) = fields[1].split(':').next_back() {
                if let (Ok(port), Ok(inode)) =
                    (u16::from_str_radix(port_hex, 16), fields[9].parse::<u64>())
                {
                    map.entry(inode).or_insert(port);
                }
            }
        }
    }

    Some(map)
}

fn read_proc_field(pid: u32, field: &str) -> String {
    std::fs::read_to_string(format!("/proc/{}/{}", pid, field))
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn read_proc_cmdline(pid: u32) -> String {
    let raw = std::fs::read_to_string(format!("/proc/{}/cmdline", pid)).unwrap_or_default();
    let cmd = raw.replace('\0', " ").trim().to_string();
    if cmd.len() > 120 {
        format!("{}...", &cmd[..117])
    } else {
        cmd
    }
}

#[cfg(test)]
mod tests {
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
        assert_eq!(detect_runtime("python3", "python3 manage.py runserver"), Runtime::Python);
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
        // On macOS this should find real ports via lsof
        // On CI/Linux this may be empty but shouldn't panic
        let ports = scan_ports();
        // Just verify it doesn't crash and returns a Vec
        let _ = ports; // just verify it doesn't panic
    }
}
