//! Linux port discovery via /proc/net/tcp and /proc/<pid>/fd symlinks.

use super::runtime::{detect_runtime, resolve_project_dir};
use super::PortInfo;
use std::collections::HashMap;

pub(super) fn scan_ports_proc() -> Vec<PortInfo> {
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
