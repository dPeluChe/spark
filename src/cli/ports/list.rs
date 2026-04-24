//! `spark ps` (no args) — list listening ports grouped into dev / macOS /
//! services / apps.

use crate::scanner::port_scanner::{self, is_dev_server, PortInfo};

pub(super) fn cmd_list_ports(show_all: bool) {
    let all_ports = port_scanner::scan_ports();
    let dev: Vec<&PortInfo> = all_ports.iter().filter(|p| is_dev_server(p)).collect();
    let sys: Vec<&PortInfo> = all_ports.iter().filter(|p| !is_dev_server(p)).collect();

    if dev.is_empty() && (!show_all || sys.is_empty()) {
        println!("\n  No dev servers running.");
        if !show_all && !sys.is_empty() {
            println!(
                "  {} system processes hidden — spark ps --all to show.",
                sys.len()
            );
        }
        return;
    }

    let all_shown: Vec<&PortInfo> = if show_all {
        all_ports.iter().collect()
    } else {
        dev.clone()
    };
    let (max_proc, max_rt) = col_widths(&all_shown);

    if !dev.is_empty() {
        print_port_section("DEV SERVERS", &dev, max_proc, max_rt);
    } else {
        println!("\n  No dev servers running.");
    }

    if show_all && !sys.is_empty() {
        let (macos, services, apps) = classify_system(&sys);
        if !macos.is_empty() {
            print_port_section("SYSTEM — macOS", &macos, max_proc, max_rt);
        }
        if !services.is_empty() {
            print_port_section("SYSTEM — SERVICES", &services, max_proc, max_rt);
        }
        if !apps.is_empty() {
            print_port_section("SYSTEM — APPS", &apps, max_proc, max_rt);
        }
    }

    println!();
    if !show_all && !sys.is_empty() {
        println!(
            "  {} system processes hidden  (spark ps --all to show)",
            sys.len()
        );
    }
    println!("  spark ps --kill <port|pid|name>  to stop a process");
}

fn classify_system<'a>(
    sys: &[&'a PortInfo],
) -> (Vec<&'a PortInfo>, Vec<&'a PortInfo>, Vec<&'a PortInfo>) {
    let mut macos = Vec::new();
    let mut services = Vec::new();
    let mut apps = Vec::new();

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
        if name == "macOS" {
            return true;
        }
    }
    let proc = p.process_name.to_lowercase();
    matches!(
        proc.as_str(),
        "controlce" | "rapportd" | "loginwindow" | "cfnetwork"
    )
}

fn is_service(p: &PortInfo) -> bool {
    if let Some(ref cwd) = p.cwd {
        if cwd.starts_with("/opt/homebrew") || cwd.starts_with("/usr/local/Cellar") {
            return true;
        }
    }
    if let port_scanner::Runtime::Other(ref name) = p.runtime {
        let n = name.to_lowercase();
        if matches!(
            n.as_str(),
            "postgresql"
                | "redis"
                | "mysql"
                | "mongodb"
                | "elasticsearch"
                | "rabbitmq"
                | "kafka"
                | "memcached"
                | "nginx"
                | "ollama"
        ) {
            return true;
        }
    }
    let proc = p.process_name.to_lowercase();
    matches!(
        proc.as_str(),
        "postgres"
            | "redis-server"
            | "redis-ser"
            | "mysqld"
            | "mongod"
            | "nginx"
            | "apache2"
            | "httpd"
            | "ollama"
            | "dnsmasq"
    )
}

fn col_widths(ports: &[&PortInfo]) -> (usize, usize) {
    let max_proc = ports
        .iter()
        .map(|p| p.process_name.len())
        .max()
        .unwrap_or(7)
        .max(16);
    let max_rt = ports
        .iter()
        .map(|p| format!("{}", p.runtime).len())
        .max()
        .unwrap_or(7)
        .max(10);
    (max_proc, max_rt)
}

fn print_port_section(label: &str, ports: &[&PortInfo], max_proc: usize, max_rt: usize) {
    const MAX_PATH: usize = 50;
    println!("\n  \x1b[1m{} ({})\x1b[0m", label, ports.len());
    println!(
        "  {:<6}  {:<7}  {:<wp$}  {:<wr$}  PROJECT",
        "PORT",
        "PID",
        "PROCESS",
        "RUNTIME",
        wp = max_proc,
        wr = max_rt
    );
    println!(
        "  {:-<6}  {:-<7}  {:-<wp$}  {:-<wr$}  {:-<30}",
        "",
        "",
        "",
        "",
        "",
        wp = max_proc,
        wr = max_rt
    );
    for p in ports {
        let raw_path = p.project_dir.as_deref().unwrap_or("-");
        let path = if raw_path.len() > MAX_PATH {
            format!("…{}", &raw_path[raw_path.len() - (MAX_PATH - 1)..])
        } else {
            raw_path.to_string()
        };
        println!(
            "  {:<6}  {:<7}  {:<wp$}  {:<wr$}  {}",
            p.port,
            p.pid,
            p.process_name,
            format!("{}", p.runtime),
            path,
            wp = max_proc,
            wr = max_rt,
        );
    }
}
