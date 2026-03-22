//! Port scanner: detect listening TCP ports and their owning processes.
//!
//! Reads /proc/net/tcp (Linux) to find listening sockets, then resolves
//! PIDs and command names via /proc/{pid}/fd and /proc/{pid}/cmdline.
//! Detects the runtime/language powering each server process.

use std::collections::HashMap;
use std::path::PathBuf;

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

/// Short colored label for each runtime
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
    3000, 3001, 3030, 3333,  // React, Next.js, Remix
    4000, 4200, 4321,        // Phoenix, Angular, Astro
    5000, 5173, 5174, 5500,  // Flask, Vite, Live Server
    6006,                     // Storybook
    8000, 8080, 8081, 8443,  // Django, generic HTTP
    8888, 8889,               // Jupyter
    9000, 9090,               // PHP, Prometheus
    9229,                     // Node debug
    19006,                    // Expo
    24678,                    // Vite HMR
];

/// Check if a port is a common dev server port
pub fn is_dev_port(port: u16) -> bool {
    DEV_PORTS.contains(&port) || (3000..=9999).contains(&port)
}

/// Scan for listening TCP ports on the system.
pub fn scan_ports() -> Vec<PortInfo> {
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
                    port,
                    pid,
                    process_name,
                    cmdline,
                    cwd,
                    runtime,
                    project_dir,
                });
            }
        }
    }

    results.sort_by_key(|p| p.port);
    results
}

/// Kill a process by PID (sends SIGTERM, then SIGKILL if needed)
pub fn kill_process(pid: u32) -> Result<(), String> {
    use std::process::Command;

    let status = Command::new("kill")
        .arg(pid.to_string())
        .status()
        .map_err(|e| format!("Failed to send SIGTERM: {}", e))?;

    if status.success() {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let still_alive = std::path::Path::new(&format!("/proc/{}", pid)).exists();
        if still_alive {
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
fn detect_runtime(process_name: &str, cmdline: &str) -> Runtime {
    let name = process_name.to_lowercase();
    let cmd = cmdline.to_lowercase();

    // Node.js variants
    if name == "node" || name.starts_with("node ") || cmd.contains("node ")
        || cmd.contains("ts-node") || cmd.contains("tsx ")
        || cmd.contains("next ") || cmd.contains("vite")
        || cmd.contains("webpack") || cmd.contains("esbuild")
        || cmd.contains("npm ") || cmd.contains("npx ")
        || cmd.contains("yarn ") || cmd.contains("pnpm ")
    {
        return Runtime::Node;
    }

    // Bun
    if name == "bun" || cmd.contains("bun ") {
        return Runtime::Bun;
    }

    // Deno
    if name == "deno" || cmd.contains("deno ") {
        return Runtime::Deno;
    }

    // Python
    if name.starts_with("python") || name == "uvicorn" || name == "gunicorn"
        || name == "flask" || name == "django" || name == "celery"
        || name == "jupyter" || name == "ipython"
        || cmd.contains("python") || cmd.contains("uvicorn")
        || cmd.contains("gunicorn") || cmd.contains("flask")
        || cmd.contains("manage.py") || cmd.contains("jupyter")
    {
        return Runtime::Python;
    }

    // Ruby
    if name == "ruby" || name == "puma" || name == "unicorn" || name == "rails"
        || cmd.contains("ruby") || cmd.contains("rails ")
        || cmd.contains("puma") || cmd.contains("unicorn")
        || cmd.contains("bundle exec")
    {
        return Runtime::Ruby;
    }

    // Java / JVM
    if name == "java" || name.starts_with("java ") || cmd.contains("java ")
        || cmd.contains("spring") || cmd.contains("gradle")
        || cmd.contains("mvn") || cmd.contains(".jar")
    {
        return Runtime::Java;
    }

    // .NET
    if name == "dotnet" || cmd.contains("dotnet ") || cmd.contains(".dll") {
        return Runtime::Dotnet;
    }

    // PHP
    if name == "php" || name == "php-fpm" || cmd.contains("php ")
        || cmd.contains("artisan") || cmd.contains("composer")
    {
        return Runtime::Php;
    }

    // Elixir / Erlang
    if name == "beam.smp" || name == "elixir" || cmd.contains("mix ")
        || cmd.contains("phoenix") || cmd.contains("elixir")
    {
        return Runtime::Elixir;
    }

    // Go (compiled binaries are trickier - check /proc/pid/exe for Go signature)
    if is_go_binary(process_name) || cmd.contains("go run") {
        return Runtime::Go;
    }

    // Rust (check if it's a cargo-run or known Rust process)
    if cmd.contains("cargo run") || cmd.contains("cargo watch") {
        return Runtime::Rust;
    }

    // nginx
    if name == "nginx" || name.starts_with("nginx") {
        return Runtime::Nginx;
    }

    // Docker
    if name == "docker-proxy" || name == "containerd" || name.starts_with("docker") {
        return Runtime::Docker;
    }

    Runtime::Other(process_name.to_string())
}

/// Try to detect Go binaries by checking if the exe has Go-like characteristics
fn is_go_binary(process_name: &str) -> bool {
    // Go binaries are statically linked and often have no extension
    // Check common Go server names
    let go_hints = [
        "air", "gin", "fiber", "echo", "mux",
    ];
    let name_lower = process_name.to_lowercase();
    go_hints.iter().any(|h| name_lower == *h)
}

/// Resolve the project directory from cwd or cmdline hints
fn resolve_project_dir(cwd: &Option<PathBuf>, cmdline: &str) -> Option<String> {
    // First try: use cwd and find nearest git root
    if let Some(dir) = cwd {
        let mut check = dir.clone();
        loop {
            if check.join(".git").exists() {
                // Return the project name (last component)
                let name = check
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                let path = check.display().to_string();
                // Shorten home prefix
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

        // No git root found, just show the cwd
        let path = dir.display().to_string();
        let home = std::env::var("HOME").unwrap_or_default();
        if path.starts_with(&home) {
            return Some(format!("~{}", &path[home.len()..]));
        }
        return Some(path);
    }

    // Fallback: try to extract path from cmdline
    for part in cmdline.split_whitespace() {
        if part.starts_with('/') || part.starts_with("./") {
            return Some(part.to_string());
        }
    }

    None
}

/// Parse /proc/net/tcp to find listening sockets.
fn parse_proc_net_tcp() -> Option<HashMap<u64, u16>> {
    let content = std::fs::read_to_string("/proc/net/tcp").ok()?;
    let mut map = HashMap::new();

    for line in content.lines().skip(1) {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 10 {
            continue;
        }

        // State 0A = LISTEN
        if fields[3] != "0A" {
            continue;
        }

        let local_addr = fields[1];
        let port_hex = local_addr.split(':').nth(1)?;
        let port = u16::from_str_radix(port_hex, 16).ok()?;
        let inode: u64 = fields[9].parse().ok()?;

        map.insert(inode, port);
    }

    // Also check /proc/net/tcp6
    if let Ok(content6) = std::fs::read_to_string("/proc/net/tcp6") {
        for line in content6.lines().skip(1) {
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 10 || fields[3] != "0A" {
                continue;
            }
            if let Some(port_hex) = fields[1].split(':').last() {
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
