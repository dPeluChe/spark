//! Render the Secrets & Credentials section of the audit report.

use super::super::shorten_path;
use crate::scanner;
use std::fmt::Write as FmtWrite;

pub(super) fn render(report: &mut String, results: &[scanner::secret_scanner::AuditResult]) {
    if results.is_empty() {
        return;
    }

    let mut total_c = 0usize;
    let mut total_w = 0usize;
    let mut total_i = 0usize;

    println!("  \x1b[1m--- Secrets & Credentials ---\x1b[0m\n");
    let _ = writeln!(report, "--- Secrets & Credentials ---\n");

    for result in results {
        total_c += result.critical_count;
        total_w += result.warning_count;
        total_i += result.info_count;

        let short = shorten_path(&result.project_path.display().to_string());
        println!(
            "  \x1b[1m{}\x1b[0m  \x1b[90m{}\x1b[0m",
            result.project_name, short
        );
        let _ = writeln!(report, "  {} ({})", result.project_name, short);

        let mut current_ctx: Option<&scanner::secret_scanner::FindingContext> = None;

        let mut i = 0;
        while i < result.findings.len() {
            let finding = &result.findings[i];

            if current_ctx != Some(&finding.context) {
                current_ctx = Some(&finding.context);
                println!("\n    \x1b[36m[{}]\x1b[0m", finding.context);
                let _ = writeln!(report, "\n    [{}]", finding.context);
            }

            // Consecutive findings sharing context + description group together
            let mut group_end = i + 1;
            while group_end < result.findings.len()
                && result.findings[group_end].context == finding.context
                && result.findings[group_end].description == finding.description
            {
                group_end += 1;
            }
            let group_size = group_end - i;

            let (icon, color) = match finding.severity {
                scanner::secret_scanner::Severity::Critical => ("!!", "31"),
                scanner::secret_scanner::Severity::Warning => ("! ", "33"),
                scanner::secret_scanner::Severity::Info => ("i ", "90"),
            };

            if group_size <= 3 {
                for f in &result.findings[i..group_end] {
                    let rel = f
                        .file_path
                        .strip_prefix(&result.project_path)
                        .unwrap_or(&f.file_path);
                    let line = if f.line_number > 0 {
                        format!(":{}", f.line_number)
                    } else {
                        String::new()
                    };
                    println!(
                        "    \x1b[{}m{}\x1b[0m  {}{}",
                        color,
                        icon,
                        rel.display(),
                        line
                    );
                    println!("        \x1b[90m-- {}\x1b[0m", f.redacted_match);
                    let _ = writeln!(report, "    {}  {}{}", icon, rel.display(), line);
                    let _ = writeln!(report, "        -- {}", f.redacted_match);
                }
            } else {
                println!(
                    "    \x1b[{}m{}\x1b[0m  {} \x1b[90m({} hits)\x1b[0m",
                    color, icon, finding.description, group_size
                );
                let _ = writeln!(
                    report,
                    "    {}  {} ({} hits)",
                    icon, finding.description, group_size
                );

                struct FileEntry {
                    line: usize,
                    redacted: String,
                }
                let mut by_file: std::collections::BTreeMap<String, Vec<FileEntry>> =
                    std::collections::BTreeMap::new();
                for f in &result.findings[i..group_end] {
                    let rel = f
                        .file_path
                        .strip_prefix(&result.project_path)
                        .unwrap_or(&f.file_path)
                        .display()
                        .to_string();
                    by_file.entry(rel).or_default().push(FileEntry {
                        line: f.line_number,
                        redacted: f.redacted_match.clone(),
                    });
                }
                for (file, entries) in &by_file {
                    if entries.len() == 1 {
                        let e = &entries[0];
                        let loc = if e.line > 0 {
                            format!(":{}", e.line)
                        } else {
                            String::new()
                        };
                        println!("        \x1b[90m{}{}\x1b[0m", file, loc);
                        println!("          \x1b[90m-- {}\x1b[0m", e.redacted);
                        let _ = writeln!(report, "        {}{}", file, loc);
                        let _ = writeln!(report, "          -- {}", e.redacted);
                    } else {
                        println!("        \x1b[90m{}\x1b[0m", file);
                        let _ = writeln!(report, "        {}", file);
                        for e in entries {
                            let loc = if e.line > 0 {
                                format!(":{}", e.line)
                            } else {
                                String::new()
                            };
                            println!("          \x1b[90m-- {} {}\x1b[0m", loc, e.redacted);
                            let _ = writeln!(report, "          -- {} {}", loc, e.redacted);
                        }
                    }
                }
            }

            i = group_end;
        }
        println!();
        let _ = writeln!(report);
    }
    print!("  ");
    let mut counts = String::new();
    if total_c > 0 {
        print!("\x1b[31m{} critical\x1b[0m  ", total_c);
        let _ = write!(counts, "{} critical  ", total_c);
    }
    if total_w > 0 {
        print!("\x1b[33m{} warnings\x1b[0m  ", total_w);
        let _ = write!(counts, "{} warnings  ", total_w);
    }
    if total_i > 0 {
        print!("\x1b[90m{} info\x1b[0m", total_i);
        let _ = write!(counts, "{} info", total_i);
    }
    println!("\n");
    let _ = writeln!(report, "  {}\n", counts.trim());
}
