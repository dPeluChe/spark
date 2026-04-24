//! `ps aux` → structured process entries. Shared between search and kill paths.

pub(super) struct PsEntry {
    pub(super) pid: u32,
    pub(super) cpu: String,
    pub(super) mem: String,
    pub(super) command: String,
    /// Short name (first token of command).
    pub(super) name: String,
}

pub(super) fn ps_list() -> Vec<PsEntry> {
    let output = std::process::Command::new("ps").args(["aux"]).output();

    let Ok(out) = output else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&out.stdout);

    text.lines()
        .skip(1)
        .filter_map(|line| {
            // ps aux cols: USER PID %CPU %MEM VSZ RSS TTY STAT START TIME COMMAND
            let mut parts = line.split_whitespace();
            let _user = parts.next()?;
            let pid: u32 = parts.next()?.parse().ok()?;
            let cpu = parts.next()?.to_string();
            let mem = parts.next()?.to_string();
            // Skip VSZ RSS TTY STAT START TIME (6 fields)
            for _ in 0..6 {
                parts.next()?;
            }
            let command: String = parts.collect::<Vec<_>>().join(" ");
            if command.is_empty() {
                return None;
            }
            let name = command
                .split('/')
                .next_back()
                .unwrap_or(&command)
                .split(' ')
                .next()
                .unwrap_or(&command)
                .to_string();
            Some(PsEntry {
                pid,
                cpu,
                mem,
                command,
                name,
            })
        })
        .collect()
}
