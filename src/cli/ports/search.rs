//! `spark ps <query>` — search processes by name, cross-ref with ports.

use super::ps_list::{ps_list, PsEntry};
use crate::scanner::port_scanner::{self, PortInfo};

pub(super) fn cmd_search(query: &str) {
    let q = query.to_lowercase();

    let procs = ps_list();
    let matched: Vec<&PsEntry> = procs
        .iter()
        .filter(|p| p.name.to_lowercase().contains(&q) || p.command.to_lowercase().contains(&q))
        .collect();

    let ports = port_scanner::scan_ports();
    let port_map: std::collections::HashMap<u32, Vec<&PortInfo>> = {
        let mut m: std::collections::HashMap<u32, Vec<&PortInfo>> =
            std::collections::HashMap::new();
        for p in &ports {
            m.entry(p.pid).or_default().push(p);
        }
        m
    };

    if matched.is_empty() {
        println!("\n  No processes matching '{}'", query);
        return;
    }

    let max_name = matched
        .iter()
        .map(|p| p.name.len())
        .max()
        .unwrap_or(7)
        .max(12);
    let max_cmd = 50usize;

    println!(
        "\n  \x1b[1mPROCESSES matching '{}' ({})\x1b[0m",
        query,
        matched.len()
    );
    println!(
        "  {:<7}  {:<5}  {:<5}  {:<wn$}  {:<wc$}  PORTS",
        "PID",
        "CPU%",
        "MEM%",
        "NAME",
        "COMMAND",
        wn = max_name,
        wc = max_cmd
    );
    println!(
        "  {:-<7}  {:-<5}  {:-<5}  {:-<wn$}  {:-<wc$}  -----",
        "",
        "",
        "",
        "",
        "",
        wn = max_name,
        wc = max_cmd
    );

    for p in &matched {
        let ports_str = match port_map.get(&p.pid) {
            Some(pp) => pp
                .iter()
                .map(|i| i.port.to_string())
                .collect::<Vec<_>>()
                .join(", "),
            None => "-".to_string(),
        };
        let cmd_display = if p.command.len() > max_cmd {
            format!("{}…", &p.command[..max_cmd - 1])
        } else {
            p.command.clone()
        };
        println!(
            "  {:<7}  {:<5}  {:<5}  {:<wn$}  {:<wc$}  {}",
            p.pid,
            p.cpu,
            p.mem,
            p.name,
            cmd_display,
            ports_str,
            wn = max_name,
            wc = max_cmd,
        );
    }

    println!();
    println!("  spark ps --kill {}  to stop", query);
}
