//! Render the OWASP code patterns section of the audit report.

use crate::scanner;
use std::fmt::Write as FmtWrite;
use std::path::Path;

struct LineHit {
    line: usize,
    text: String,
}

struct IssueGroup {
    cat: String,
    severity: scanner::code_patterns::PatternSeverity,
    suggestion: String,
    files: std::collections::BTreeMap<String, Vec<LineHit>>,
}

pub(super) fn render(
    report: &mut String,
    patterns: &[scanner::code_patterns::PatternFinding],
    scan_path: &Path,
) {
    if patterns.is_empty() {
        return;
    }

    println!("  \x1b[1m--- Code Patterns (OWASP Top 10:2025) ---\x1b[0m");
    println!("  \x1b[90mRef: owasp.org/Top10 — categories shown per finding\x1b[0m");
    println!("  \x1b[90mA01 Access Control · A03 Injection · A04 Crypto · A05 Misconfig · A08 Integrity\x1b[0m\n");

    let _ = writeln!(
        report,
        "--- Code Patterns (OWASP Top 10:2025 — owasp.org/Top10) ---"
    );
    let _ = writeln!(
        report,
        "A01 Broken Access Control · A03 Injection (SQL/Cmd/XSS) · A04 Cryptographic Failures"
    );
    let _ = writeln!(
        report,
        "A05 Security Misconfiguration · A08 Software & Data Integrity Failures\n"
    );

    let mut by_issue: Vec<(String, IssueGroup)> = Vec::new();

    for pf in patterns {
        let rel = pf
            .file_path
            .strip_prefix(scan_path)
            .unwrap_or(&pf.file_path)
            .display()
            .to_string();
        let hit = LineHit {
            line: pf.line_number,
            text: pf.matched_text.clone(),
        };
        if let Some((_, group)) = by_issue.iter_mut().find(|(d, _)| d == &pf.description) {
            group.files.entry(rel).or_default().push(hit);
        } else {
            let mut files = std::collections::BTreeMap::new();
            files.insert(rel, vec![hit]);
            by_issue.push((
                pf.description.clone(),
                IssueGroup {
                    cat: format!("{}", pf.category),
                    severity: pf.severity,
                    suggestion: pf.suggestion.clone(),
                    files,
                },
            ));
        }
    }

    for (desc, group) in &by_issue {
        render_issue_group(report, desc, group);
    }

    render_severity_summary(report, patterns);
}

fn render_issue_group(report: &mut String, desc: &str, group: &IssueGroup) {
    let (icon, color) = match group.severity {
        scanner::code_patterns::PatternSeverity::High => ("!!", "31"),
        scanner::code_patterns::PatternSeverity::Medium => ("! ", "33"),
        scanner::code_patterns::PatternSeverity::Low => ("i ", "90"),
    };
    let total_hits: usize = group.files.values().map(|v| v.len()).sum();
    println!(
        "    \x1b[36m[{}]\x1b[0m \x1b[{}m{}\x1b[0m {} \x1b[90m({} hits in {} files)\x1b[0m",
        group.cat,
        color,
        icon,
        desc,
        total_hits,
        group.files.len()
    );
    println!("        \x1b[90m-> {}\x1b[0m", group.suggestion);
    let _ = writeln!(
        report,
        "    [{}] {} {} ({} hits in {} files)",
        group.cat,
        icon,
        desc,
        total_hits,
        group.files.len()
    );
    let _ = writeln!(report, "        -> {}", group.suggestion);

    if group.files.len() <= 8 {
        for (file, hits) in &group.files {
            render_file_hits(report, file, hits);
        }
    } else {
        render_grouped_by_dir(report, group);
    }
    println!();
    let _ = writeln!(report);
}

fn render_file_hits(report: &mut String, file: &str, hits: &[LineHit]) {
    if hits.len() == 1 {
        println!("        \x1b[90m{}:{}\x1b[0m", file, hits[0].line);
        println!("          \x1b[90m-- {}\x1b[0m", hits[0].text);
        let _ = writeln!(report, "        {}:{}", file, hits[0].line);
        let _ = writeln!(report, "          -- {}", hits[0].text);
    } else {
        println!("        \x1b[90m{}\x1b[0m", file);
        let _ = writeln!(report, "        {}", file);
        for h in hits {
            println!("          \x1b[90m-- :{} {}\x1b[0m", h.line, h.text);
            let _ = writeln!(report, "          -- :{} {}", h.line, h.text);
        }
    }
}

fn render_grouped_by_dir(report: &mut String, group: &IssueGroup) {
    let mut by_dir: std::collections::BTreeMap<String, Vec<(&String, &Vec<LineHit>)>> =
        std::collections::BTreeMap::new();
    for (file, hits) in &group.files {
        let dir = std::path::Path::new(file)
            .parent()
            .map(|p| {
                let s = p.display().to_string();
                let parts: Vec<&str> = s.split('/').collect();
                let depth = parts.len().min(3);
                parts[..depth].join("/")
            })
            .unwrap_or_else(|| ".".into());
        by_dir.entry(dir).or_default().push((file, hits));
    }

    for (dir, files_in_dir) in &by_dir {
        let dir_hits: usize = files_in_dir.iter().map(|(_, h)| h.len()).sum();
        println!(
            "\n        \x1b[33m{}/\x1b[0m \x1b[90m({} files, {} hits)\x1b[0m",
            dir,
            files_in_dir.len(),
            dir_hits
        );
        let _ = writeln!(
            report,
            "\n        {}/ ({} files, {} hits)",
            dir,
            files_in_dir.len(),
            dir_hits
        );
        for (file, hits) in files_in_dir {
            let short = file.strip_prefix(&format!("{}/", dir)).unwrap_or(file);
            if hits.len() == 1 {
                println!("          \x1b[90m{}:{}\x1b[0m", short, hits[0].line);
                println!("            \x1b[90m-- {}\x1b[0m", hits[0].text);
                let _ = writeln!(report, "          {}:{}", short, hits[0].line);
                let _ = writeln!(report, "            -- {}", hits[0].text);
            } else {
                println!("          \x1b[90m{}\x1b[0m", short);
                let _ = writeln!(report, "          {}", short);
                for h in *hits {
                    println!("            \x1b[90m-- :{} {}\x1b[0m", h.line, h.text);
                    let _ = writeln!(report, "            -- :{} {}", h.line, h.text);
                }
            }
        }
    }
}

fn render_severity_summary(
    report: &mut String,
    patterns: &[scanner::code_patterns::PatternFinding],
) {
    let high = patterns
        .iter()
        .filter(|p| p.severity == scanner::code_patterns::PatternSeverity::High)
        .count();
    let med = patterns
        .iter()
        .filter(|p| p.severity == scanner::code_patterns::PatternSeverity::Medium)
        .count();
    let low = patterns
        .iter()
        .filter(|p| p.severity == scanner::code_patterns::PatternSeverity::Low)
        .count();
    print!("  ");
    let mut counts = String::new();
    if high > 0 {
        print!("\x1b[31m{} high\x1b[0m  ", high);
        let _ = write!(counts, "{} high  ", high);
    }
    if med > 0 {
        print!("\x1b[33m{} medium\x1b[0m  ", med);
        let _ = write!(counts, "{} medium  ", med);
    }
    if low > 0 {
        print!("\x1b[90m{} low\x1b[0m", low);
        let _ = write!(counts, "{} low", low);
    }
    println!();
    let _ = writeln!(report, "  {}", counts.trim());
}
