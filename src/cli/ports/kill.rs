//! `spark ps --kill` — interactive and non-interactive kill by port, PID, or name.

use super::ps_list::{ps_list, PsEntry};
use crate::scanner::port_scanner::{self, PortInfo};

/// Non-interactive kill. Used when query + --kill are combined. Exits 0/1.
pub(super) fn cmd_kill_silent(target: &str) {
    let ports = port_scanner::scan_ports();

    if let Ok(num) = target.parse::<u32>() {
        let found = find_by_number(&ports, num);
        if !found.is_empty() {
            let mut killed = false;
            for p in found {
                match port_scanner::kill_process(p.pid) {
                    Ok(_) => {
                        println!(
                            "  \x1b[32m[+]\x1b[0m Killed {} (pid {})",
                            p.process_name, p.pid
                        );
                        killed = true;
                    }
                    Err(e) => eprintln!("  \x1b[31m[!]\x1b[0m Failed: {}", e),
                }
            }
            if !killed {
                std::process::exit(1);
            }
            return;
        }
        if port_scanner::kill_process(num).is_ok() {
            println!("  \x1b[32m[+]\x1b[0m Killed pid {}", num);
            return;
        }
        eprintln!("  process not found: {}", target);
        std::process::exit(1);
    }

    let q = target.to_lowercase();
    let by_name: Vec<&PortInfo> = ports
        .iter()
        .filter(|p| p.process_name.to_lowercase().contains(&q))
        .collect();

    let own_pid = std::process::id();
    let pids: Vec<u32> = if !by_name.is_empty() {
        by_name
            .iter()
            .map(|p| p.pid)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    } else {
        // Fall back to ps aux — exclude ourselves (our argv contains the query string)
        ps_list()
            .into_iter()
            .filter(|p| p.pid != own_pid)
            .filter(|p| p.name.to_lowercase().contains(&q) || p.command.to_lowercase().contains(&q))
            .map(|p| p.pid)
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    };

    if pids.is_empty() {
        eprintln!("  process not found: {}", target);
        std::process::exit(1);
    }

    let mut killed = false;
    for pid in pids {
        match port_scanner::kill_process(pid) {
            Ok(_) => {
                println!("  \x1b[32m[+]\x1b[0m Killed {} (pid {})", target, pid);
                killed = true;
            }
            Err(e) => eprintln!("  \x1b[31m[!]\x1b[0m Failed to kill pid {}: {}", pid, e),
        }
    }
    if !killed {
        std::process::exit(1);
    }
}

/// Interactive kill — prompts per process.
pub(super) fn cmd_kill(target: &str) {
    let ports = port_scanner::scan_ports();

    if let Ok(num) = target.parse::<u32>() {
        let found = find_by_number(&ports, num);
        if !found.is_empty() {
            kill_port_entries(&found);
            return;
        }
        kill_pid_direct(num, target);
        return;
    }

    let q = target.to_lowercase();
    let by_name: Vec<&PortInfo> = ports
        .iter()
        .filter(|p| p.process_name.to_lowercase().contains(&q))
        .collect();

    if !by_name.is_empty() {
        kill_port_entries(&by_name);
        return;
    }

    let own_pid = std::process::id();
    let procs = ps_list();
    let matched: Vec<&PsEntry> = procs
        .iter()
        .filter(|p| p.pid != own_pid)
        .filter(|p| p.name.to_lowercase().contains(&q) || p.command.to_lowercase().contains(&q))
        .collect();

    if matched.is_empty() {
        eprintln!("  No process found matching: {}", target);
        std::process::exit(1);
    }

    for p in matched {
        let cmd_short = if p.command.len() > 60 {
            format!("{}…", &p.command[..59])
        } else {
            p.command.clone()
        };
        print!(
            "  Kill \x1b[1m{}\x1b[0m (pid {})  {}? [y/N]: ",
            p.name, p.pid, cmd_short
        );
        flush_confirm();
        if confirmed() {
            kill_pid_direct(p.pid, &p.name);
        } else {
            println!("  [-] Skipped");
        }
    }
}

fn find_by_number(ports: &[PortInfo], num: u32) -> Vec<&PortInfo> {
    if num <= 65535 {
        let by_port: Vec<&PortInfo> = ports.iter().filter(|p| p.port == num as u16).collect();
        if !by_port.is_empty() {
            return by_port;
        }
    }
    ports.iter().filter(|p| p.pid == num).collect()
}

fn kill_port_entries(found: &[&PortInfo]) {
    for p in found {
        print!(
            "  Kill \x1b[1m{}\x1b[0m (pid {}) on port {}? [y/N]: ",
            p.process_name, p.pid, p.port
        );
        flush_confirm();
        if confirmed() {
            match port_scanner::kill_process(p.pid) {
                Ok(_) => println!(
                    "  \x1b[32m[+]\x1b[0m Killed {} (pid {})",
                    p.process_name, p.pid
                ),
                Err(e) => eprintln!("  \x1b[31m[!]\x1b[0m Failed: {}", e),
            }
        } else {
            println!("  [-] Skipped");
        }
    }
}

fn kill_pid_direct(pid: u32, label: &str) {
    match port_scanner::kill_process(pid) {
        Ok(_) => println!("  \x1b[32m[+]\x1b[0m Killed {} (pid {})", label, pid),
        Err(e) => eprintln!("  \x1b[31m[!]\x1b[0m Failed to kill {}: {}", label, e),
    }
}

fn flush_confirm() {
    use std::io::Write;
    std::io::stdout().flush().ok();
}

fn confirmed() -> bool {
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer).ok();
    answer.trim().to_lowercase() == "y"
}
