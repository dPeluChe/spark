//! `spark ps` — unified process + port inspector.
//!
//! - No query:      show listening ports (dev servers by default)
//! - With query:    search running processes by name + show their ports if any
//! - `--kill`:      kill by port number, PID, or process name

mod kill;
mod list;
mod ps_list;
mod search;

pub fn cmd_ports(show_all: bool, query: Option<String>, kill_target: Option<String>) {
    match (query, kill_target) {
        // spark ps <query> --kill  → non-interactive kill, exit 0/1
        (Some(q), Some(k)) if k.is_empty() => kill::cmd_kill_silent(&q),
        // spark ps --kill <target> → interactive kill
        (None, Some(target)) => kill::cmd_kill(&target),
        // spark ps <query>         → search processes
        (Some(q), None) => search::cmd_search(&q),
        // spark ps [--all]         → list ports
        _ => list::cmd_list_ports(show_all),
    }
}
