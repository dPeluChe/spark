//! spark ps — unified process + port inspector.
//!
//! No query:      show listening ports (dev servers by default)
//! With query:    search running processes by name + show their ports if any
//! --kill target: kill by port number, PID, or process name

use crate::scanner::port_scanner::{self, PortInfo, is_dev_server};

/// A running process from `ps aux`
struct PsEntry {
    pid:     u32,
    cpu:     String,
    mem:     String,
    command: String,
    /// Short name (first token of command)
    name:    String,
}

pub fn cmd_ports(show_all: bool, query: Option<String>, kill: Option<String>) {
    match (query, kill) {
        // spark ps <query> --kill  → non-interactive kill, exit 0/1
        (Some(q), Some(k)) if k.is_empty() => cmd_kill_silent(&q),
        // spark ps --kill <target> → interactive kill
        (None, Some(target)) => cmd_kill(&target),
        // spark ps <query>         → search processes
        (Some(q), None) => cmd_search(&q),
        // spark ps [--all]         → list ports
        _ => cmd_list_ports(show_all),
    }
}

// ─── List ports (default) ────────────────────────────────────────────────────

fn cmd_list_ports(show_all: bool) {
    let all_ports = port_scanner::scan_ports();
    let dev: Vec<&PortInfo> = all_ports.iter().filter(|p| is_dev_server(p)).collect();
    let sys: Vec<&PortInfo> = all_ports.iter().filter(|p| !is_dev_server(p)).collect();

    if dev.is_empty() && (!show_all || sys.is_empty()) {
        println!("\n  No dev servers running.");
        if !show_all && !sys.is_empty() {
            println!("  {} system processes hidden — spark ps --all to show.", sys.len());
        }
        return;
    }

    let all_shown: Vec<&PortInfo> = if show_all { all_ports.iter().collect() } else { dev.clone() };
    let (max_proc, max_rt) = col_widths(&all_shown);

    if !dev.is_empty() {
        print_port_section("DEV SERVERS", &dev, max_proc, max_rt);
    } else {
        println!("\n  No dev servers running.");
    }

    if show_all && !sys.is_empty() {
        let (macos, services, apps) = classify_system(&sys);
        if !macos.is_empty()    { print_port_section("SYSTEM — macOS", &macos, max_proc, max_rt); }
        if !services.is_empty() { print_port_section("SYSTEM — SERVICES", &services, max_proc, max_rt); }
        if !apps.is_empty()     { print_port_section("SYSTEM — APPS", &apps, max_proc, max_rt); }
    }

    println!();
    if !show_all && !sys.is_empty() {
        println!("  {} system processes hidden  (spark ps --all to show)", sys.len());
    }
    println!("  spark ps --kill <port|pid|name>  to stop a process");
}

fn classify_system<'a>(sys: &[&'a PortInfo]) -> (Vec<&'a PortInfo>, Vec<&'a PortInfo>, Vec<&'a PortInfo>) {
    let mut macos    = Vec::new();
    let mut services = Vec::new();
    let mut apps     = Vec::new();

    for &p in sys {
        if is_macos_system(p) {
            macos.push(p);
        } else if is_service(p) {
            services.push(p);
        } else {
            apps.push(p);
        }
    }
    (macos, services, apps)
}

fn is_macos_system(p: &PortInfo) -> bool {
    if let port_scanner::Runtime::Other(ref name) = p.runtime {
        if name == "macOS" { return true; }
    }
    let proc = p.process_name.to_lowercase();
    matches!(proc.as_str(), "controlce" | "rapportd" | "loginwindow" | "cfnetwork")
}

fn is_service(p: &PortInfo) -> bool {
    // Homebrew-installed services
    if let Some(ref cwd) = p.cwd {
        if cwd.starts_with("/opt/homebrew") || cwd.starts_with("/usr/local/Cellar") {
            return true;
        }
    }
    // Known data/infra services by runtime or name
    if let port_scanner::Runtime::Other(ref name) = p.runtime {
        let n = name.to_lowercase();
        if matches!(n.as_str(),
            "postgresql" | "redis" | "mysql" | "mongodb" | "elasticsearch"
            | "rabbitmq" | "kafka" | "memcached" | "nginx" | "ollama"
        ) {
            return true;
        }
    }
    let proc = p.process_name.to_lowercase();
    matches!(proc.as_str(),
        "postgres" | "redis-server" | "redis-ser" | "mysqld" | "mongod"
        | "nginx" | "apache2" | "httpd" | "ollama" | "dnsmasq"
    )
}

fn col_widths(ports: &[&PortInfo]) -> (usize, usize) {
    let max_proc = ports.iter().map(|p| p.process_name.len()).max().unwrap_or(7).max(16);
    let max_rt   = ports.iter().map(|p| format!("{}", p.runtime).len()).max().unwrap_or(7).max(10);
    (max_proc, max_rt)
}

fn print_port_section(label: &str, ports: &[&PortInfo], max_proc: usize, max_rt: usize) {
    println!("\n  \x1b[1m{} ({})\x1b[0m", label, ports.len());
    println!("  {:<6}  {:<7}  {:<wp$}  {:<wr$}  PROJECT", "PORT", "PID", "PROCESS", "RUNTIME", wp = max_proc, wr = max_rt);
    println!("  {:-<6}  {:-<7}  {:-<wp$}  {:-<wr$}  {:-<30}", "", "", "", "", "", wp = max_proc, wr = max_rt);
    for p in ports {
        println!(
            "  {:<6}  {:<7}  {:<wp$}  {:<wr$}  {}",
            p.port, p.pid, p.process_name,
            format!("{}", p.runtime),
            p.project_dir.as_deref().unwrap_or("-"),
            wp = max_proc, wr = max_rt,
        );
    }
}

// ─── Search processes by name ─────────────────────────────────────────────────

fn cmd_search(query: &str) {
    let q = query.to_lowercase();

    // Get all running processes from ps
    let procs = ps_list();
    let matched: Vec<&PsEntry> = procs.iter()
        .filter(|p| p.name.to_lowercase().contains(&q) || p.command.to_lowercase().contains(&q))
        .collect();

    // Get listening ports for cross-reference
    let ports = port_scanner::scan_ports();
    let port_map: std::collections::HashMap<u32, Vec<&PortInfo>> = {
        let mut m: std::collections::HashMap<u32, Vec<&PortInfo>> = std::collections::HashMap::new();
        for p in &ports {
            m.entry(p.pid).or_default().push(p);
        }
        m
    };

    if matched.is_empty() {
        println!("\n  No processes matching '{}'", query);
        return;
    }

    let max_name = matched.iter().map(|p| p.name.len()).max().unwrap_or(7).max(12);
    let max_cmd  = 50usize;

    println!("\n  \x1b[1mPROCESSES matching '{}' ({})\x1b[0m", query, matched.len());
    println!("  {:<7}  {:<5}  {:<5}  {:<wn$}  {:<wc$}  PORTS", "PID", "CPU%", "MEM%", "NAME", "COMMAND", wn = max_name, wc = max_cmd);
    println!("  {:-<7}  {:-<5}  {:-<5}  {:-<wn$}  {:-<wc$}  -----", "", "", "", "", "", wn = max_name, wc = max_cmd);

    for p in &matched {
        let ports_str = match port_map.get(&p.pid) {
            Some(pp) => pp.iter().map(|i| i.port.to_string()).collect::<Vec<_>>().join(", "),
            None => "-".to_string(),
        };
        let cmd_display = if p.command.len() > max_cmd {
            format!("{}…", &p.command[..max_cmd - 1])
        } else {
            p.command.clone()
        };
        println!(
            "  {:<7}  {:<5}  {:<5}  {:<wn$}  {:<wc$}  {}",
            p.pid, p.cpu, p.mem, p.name, cmd_display, ports_str,
            wn = max_name, wc = max_cmd,
        );
    }

    println!();
    println!("  spark ps --kill {}  to stop", query);
}

fn ps_list() -> Vec<PsEntry> {
    let output = std::process::Command::new("ps")
        .args(["aux"])
        .output();

    let Ok(out) = output else { return Vec::new(); };
    let text = String::from_utf8_lossy(&out.stdout);

    text.lines().skip(1).filter_map(|line| {
        // ps aux cols: USER PID %CPU %MEM VSZ RSS TTY STAT START TIME COMMAND
        // Use split_whitespace to handle variable spacing between columns.
        let mut parts = line.split_whitespace();
        let _user = parts.next()?;
        let pid: u32 = parts.next()?.parse().ok()?;
        let cpu = parts.next()?.to_string();
        let mem = parts.next()?.to_string();
        // skip VSZ RSS TTY STAT START TIME (6 fields)
        for _ in 0..6 { parts.next()?; }
        let command: String = parts.collect::<Vec<_>>().join(" ");
        if command.is_empty() { return None; }
        let name = command.split('/').next_back().unwrap_or(&command)
            .split(' ').next().unwrap_or(&command)
            .to_string();
        Some(PsEntry { pid, cpu, mem, command, name })
    }).collect()
}

// ─── Kill by port, PID, or process name ──────────────────────────────────────

/// Non-interactive kill — used when query + --kill are combined. Exits 0/1.
fn cmd_kill_silent(target: &str) {
    let ports = port_scanner::scan_ports();

    if let Ok(num) = target.parse::<u32>() {
        let found: Vec<&PortInfo> = if num <= 65535 {
            let by_port: Vec<&PortInfo> = ports.iter().filter(|p| p.port == num as u16).collect();
            if !by_port.is_empty() { by_port } else { ports.iter().filter(|p| p.pid == num).collect() }
        } else {
            ports.iter().filter(|p| p.pid == num).collect()
        };

        if !found.is_empty() {
            let mut killed = false;
            for p in found {
                match port_scanner::kill_process(p.pid) {
                    Ok(_) => { println!("  \x1b[32m[+]\x1b[0m Killed {} (pid {})", p.process_name, p.pid); killed = true; }
                    Err(e) => eprintln!("  \x1b[31m[!]\x1b[0m Failed: {}", e),
                }
            }
            if !killed { std::process::exit(1); }
            return;
        }
        // Try direct PID kill
        if port_scanner::kill_process(num).is_ok() {
            println!("  \x1b[32m[+]\x1b[0m Killed pid {}", num);
            return;
        }
        eprintln!("  process not found: {}", target);
        std::process::exit(1);
    }

    // Name: search port list first
    let q = target.to_lowercase();
    let by_name: Vec<&PortInfo> = ports.iter()
        .filter(|p| p.process_name.to_lowercase().contains(&q))
        .collect();

    let own_pid = std::process::id();
    let pids: Vec<u32> = if !by_name.is_empty() {
        by_name.iter().map(|p| p.pid).collect::<std::collections::HashSet<_>>().into_iter().collect()
    } else {
        // Fall back to ps aux — exclude ourselves (our argv contains the query string)
        ps_list().into_iter()
            .filter(|p| p.pid != own_pid)
            .filter(|p| p.name.to_lowercase().contains(&q) || p.command.to_lowercase().contains(&q))
            .map(|p| p.pid)
            .collect::<std::collections::HashSet<_>>().into_iter().collect()
    };

    if pids.is_empty() {
        eprintln!("  process not found: {}", target);
        std::process::exit(1);
    }

    let mut killed = false;
    for pid in pids {
        match port_scanner::kill_process(pid) {
            Ok(_) => { println!("  \x1b[32m[+]\x1b[0m Killed {} (pid {})", target, pid); killed = true; }
            Err(e) => eprintln!("  \x1b[31m[!]\x1b[0m Failed to kill pid {}: {}", pid, e),
        }
    }
    if !killed { std::process::exit(1); }
}

fn cmd_kill(target: &str) {
    let ports = port_scanner::scan_ports();

    // Numeric: try as port, then PID
    if let Ok(num) = target.parse::<u32>() {
        let found: Vec<&PortInfo> = if num <= 65535 {
            let by_port: Vec<&PortInfo> = ports.iter().filter(|p| p.port == num as u16).collect();
            if !by_port.is_empty() { by_port } else { ports.iter().filter(|p| p.pid == num).collect() }
        } else {
            ports.iter().filter(|p| p.pid == num).collect()
        };

        if !found.is_empty() {
            kill_port_entries(&found);
            return;
        }

        // PID not in port list — try killing directly
        kill_pid_direct(num, target);
        return;
    }

    // Name: search ports list first, then ps aux
    let q = target.to_lowercase();
    let by_name: Vec<&PortInfo> = ports.iter()
        .filter(|p| p.process_name.to_lowercase().contains(&q))
        .collect();

    if !by_name.is_empty() {
        kill_port_entries(&by_name);
        return;
    }

    // Fall back to ps aux search — exclude ourselves
    let own_pid = std::process::id();
    let procs = ps_list();
    let matched: Vec<&PsEntry> = procs.iter()
        .filter(|p| p.pid != own_pid)
        .filter(|p| p.name.to_lowercase().contains(&q) || p.command.to_lowercase().contains(&q))
        .collect();

    if matched.is_empty() {
        eprintln!("  No process found matching: {}", target);
        std::process::exit(1);
    }

    for p in matched {
        let cmd_short = if p.command.len() > 60 { format!("{}…", &p.command[..59]) } else { p.command.clone() };
        print!("  Kill \x1b[1m{}\x1b[0m (pid {})  {}? [y/N]: ", p.name, p.pid, cmd_short);
        flush_confirm();
        if confirmed() {
            kill_pid_direct(p.pid, &p.name);
        } else {
            println!("  [-] Skipped");
        }
    }
}

fn kill_port_entries(found: &[&PortInfo]) {
    for p in found {
        print!("  Kill \x1b[1m{}\x1b[0m (pid {}) on port {}? [y/N]: ", p.process_name, p.pid, p.port);
        flush_confirm();
        if confirmed() {
            match port_scanner::kill_process(p.pid) {
                Ok(_) => println!("  \x1b[32m[+]\x1b[0m Killed {} (pid {})", p.process_name, p.pid),
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
