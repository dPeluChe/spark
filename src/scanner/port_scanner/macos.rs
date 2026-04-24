//! macOS port discovery: `lsof -iTCP -sTCP:LISTEN` plus batched `ps` + `lsof` for metadata.

use super::runtime::{detect_runtime, resolve_project_dir};
use super::PortInfo;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

pub(super) fn scan_ports_lsof() -> Vec<PortInfo> {
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

    struct RawEntry {
        port: u16,
        pid: u32,
        process_name: String,
    }
    let mut seen_ports: HashMap<u16, usize> = HashMap::new();
    let mut entries: Vec<RawEntry> = Vec::new();

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
        let port: u16 = match fields[8].rsplit(':').next().and_then(|p| p.parse().ok()) {
            Some(p) => p,
            None => continue,
        };

        if let Some(&idx) = seen_ports.get(&port) {
            if entries[idx].pid == pid {
                continue; // IPv4/IPv6 duplicate for same PID
            }
            entries[idx] = RawEntry {
                port,
                pid,
                process_name,
            };
        } else {
            seen_ports.insert(port, entries.len());
            entries.push(RawEntry {
                port,
                pid,
                process_name,
            });
        }
    }

    if entries.is_empty() {
        return Vec::new();
    }

    let mut unique_pids: Vec<u32> = entries.iter().map(|e| e.pid).collect();
    unique_pids.sort_unstable();
    unique_pids.dedup();

    let cmdlines = get_cmdlines_batch(&unique_pids);
    let cwds = get_cwds_batch(&unique_pids);

    let mut results: Vec<PortInfo> = entries
        .into_iter()
        .map(|e| {
            let cmdline = cmdlines.get(&e.pid).cloned().unwrap_or_default();
            let cwd = cwds.get(&e.pid).cloned();
            let runtime = detect_runtime(&e.process_name, &cmdline);
            let project_dir = resolve_project_dir(&cwd, &cmdline);
            PortInfo {
                port: e.port,
                pid: e.pid,
                process_name: e.process_name,
                cmdline,
                cwd,
                runtime,
                project_dir,
            }
        })
        .collect();

    results.sort_by_key(|p| p.port);
    results
}

fn pid_list(pids: &[u32]) -> String {
    pids.iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

/// Batch: command lines for all PIDs in one `ps` call.
fn get_cmdlines_batch(pids: &[u32]) -> HashMap<u32, String> {
    if pids.is_empty() {
        return HashMap::new();
    }
    let output = match Command::new("ps")
        .args(["-p", &pid_list(pids), "-o", "pid=,command="])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return HashMap::new(),
    };
    let mut map = HashMap::new();
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(idx) = line.find(|c: char| c.is_ascii_whitespace()) {
            if let Ok(pid) = line[..idx].trim().parse::<u32>() {
                let cmd = line[idx..].trim().to_string();
                let cmd = if cmd.len() > 120 {
                    format!("{}...", crate::scanner::common::safe_truncate(&cmd, 117))
                } else {
                    cmd
                };
                map.insert(pid, cmd);
            }
        }
    }
    map
}

/// Batch: working directories for all PIDs in one `lsof` call.
fn get_cwds_batch(pids: &[u32]) -> HashMap<u32, PathBuf> {
    if pids.is_empty() {
        return HashMap::new();
    }
    let output = match Command::new("lsof")
        .args(["-a", "-p", &pid_list(pids), "-d", "cwd", "-Fn"])
        .output()
    {
        Ok(o) if o.status.success() => o,
        _ => return HashMap::new(),
    };
    let mut map = HashMap::new();
    let mut current_pid: Option<u32> = None;
    for line in String::from_utf8_lossy(&output.stdout).lines() {
        if let Some(rest) = line.strip_prefix('p') {
            current_pid = rest.parse().ok();
        } else if let Some(path) = line.strip_prefix('n') {
            if path.starts_with('/') {
                if let Some(pid) = current_pid {
                    map.insert(pid, PathBuf::from(path));
                }
            }
        }
    }
    map
}
