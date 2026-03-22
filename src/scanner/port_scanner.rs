//! Port scanner: detect listening TCP ports and their owning processes.
//!
//! Reads /proc/net/tcp (Linux) to find listening sockets, then resolves
//! PIDs and command names via /proc/{pid}/fd and /proc/{pid}/cmdline.

use std::collections::HashMap;
use std::path::PathBuf;

/// Information about a listening port
#[derive(Debug, Clone)]
pub struct PortInfo {
    pub port: u16,
    pub pid: u32,
    pub process_name: String,
    pub cmdline: String,
    pub cwd: Option<PathBuf>,
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
/// Returns a sorted list of PortInfo for ports in the dev range.
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

    // Scan /proc for processes owning these inodes
    let proc_entries = match std::fs::read_dir("/proc") {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    for entry in proc_entries.filter_map(|e| e.ok()) {
        let pid: u32 = match entry.file_name().to_str().and_then(|s| s.parse().ok()) {
            Some(pid) => pid,
            None => continue,
        };

        // Check this process's file descriptors for matching inodes
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

            // Extract inode from "socket:[12345]"
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

                results.push(PortInfo {
                    port,
                    pid,
                    process_name,
                    cmdline,
                    cwd,
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
        // Give it a moment, then check if still alive
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

/// Parse /proc/net/tcp to find listening sockets.
/// Returns a map of inode -> port for sockets in LISTEN state.
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

        // local_address is in hex: "0100007F:1F90" -> 127.0.0.1:8080
        let local_addr = fields[1];
        let port_hex = local_addr.split(':').nth(1)?;
        let port = u16::from_str_radix(port_hex, 16).ok()?;

        // inode
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
    // cmdline uses NUL separators
    let cmd = raw.replace('\0', " ").trim().to_string();
    // Truncate for display
    if cmd.len() > 120 {
        format!("{}...", &cmd[..117])
    } else {
        cmd
    }
}
