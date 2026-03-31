//! Security audit CLI command — secrets, git history, and OWASP code patterns.

use std::path::PathBuf;
use std::fmt::Write as FmtWrite;
use crate::scanner;
use super::shorten_path;

pub fn cmd_audit(path: Option<PathBuf>, output_file: Option<PathBuf>, init_ignore: bool) {
    let scan_path = path.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    if init_ignore {
        create_ignore_file(&scan_path);
        return;
    }

    println!("  SPARK Security Audit");
    println!("  Everything runs locally — nothing leaves your machine.\n");

    // Phase 1: Secret scan (files)
    eprint!("  [1/3] Secrets scan");
    let results = scanner::secret_scanner::scan_directory_with_progress(
        &scan_path,
        Some(&|count| { if count % 50 == 0 { eprint!("."); } }),
    );
    eprintln!(" done");

    // Phase 2: Git history scan
    eprint!("  [2/3] Git history scan");
    let has_git = scan_path.join(".git").exists();
    let history = if has_git {
        let h = scanner::history_scanner::scan_history(&scan_path);
        eprintln!(".. {} findings", h.len());
        h
    } else {
        eprintln!(".. skipped (no .git)");
        Vec::new()
    };

    // Phase 3: OWASP code patterns
    eprint!("  [3/3] Code patterns scan");
    let patterns = scanner::code_patterns::scan_code_patterns(&scan_path);
    eprintln!(".. {} findings", patterns.len());
    println!();

    let has_findings = !results.is_empty() || !history.is_empty() || !patterns.is_empty();
    if !has_findings {
        println!("  \x1b[32mNo security findings detected.\x1b[0m");
        return;
    }

    // Build the report (both for terminal and file output)
    let mut report = String::new();

    // ── Secrets (grouped by context, then by description) ──
    if !results.is_empty() {
        let mut total_c = 0usize;
        let mut total_w = 0usize;
        let mut total_i = 0usize;

        println!("  \x1b[1m--- Secrets & Credentials ---\x1b[0m\n");
        let _ = writeln!(report, "--- Secrets & Credentials ---\n");

        for result in &results {
            total_c += result.critical_count;
            total_w += result.warning_count;
            total_i += result.info_count;

            let short = shorten_path(&result.project_path.display().to_string());
            println!("  \x1b[1m{}\x1b[0m  \x1b[90m{}\x1b[0m", result.project_name, short);
            let _ = writeln!(report, "  {} ({})", result.project_name, short);

            // Group by context -> description -> files
            let mut current_ctx: Option<&scanner::secret_scanner::FindingContext> = None;

            // Collect findings that share the same context+description for grouping
            let mut i = 0;
            while i < result.findings.len() {
                let finding = &result.findings[i];

                // Print context header when it changes
                if current_ctx != Some(&finding.context) {
                    current_ctx = Some(&finding.context);
                    println!("\n    \x1b[36m[{}]\x1b[0m", finding.context);
                    let _ = writeln!(report, "\n    [{}]", finding.context);
                }

                // Count consecutive findings with same context + description
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
                        let rel = f.file_path.strip_prefix(&result.project_path).unwrap_or(&f.file_path);
                        let line = if f.line_number > 0 { format!(":{}", f.line_number) } else { String::new() };
                        println!("    \x1b[{}m{}\x1b[0m  {}{}",
                            color, icon, rel.display(), line);
                        println!("        \x1b[90m-- {}\x1b[0m", f.redacted_match);
                        let _ = writeln!(report, "    {}  {}{}", icon, rel.display(), line);
                        let _ = writeln!(report, "        -- {}", f.redacted_match);
                    }
                } else {
                    println!("    \x1b[{}m{}\x1b[0m  {} \x1b[90m({} hits)\x1b[0m",
                        color, icon, finding.description, group_size);
                    let _ = writeln!(report, "    {}  {} ({} hits)", icon, finding.description, group_size);

                    struct FileEntry { line: usize, redacted: String }
                    let mut by_file: std::collections::BTreeMap<String, Vec<FileEntry>> = std::collections::BTreeMap::new();
                    for f in &result.findings[i..group_end] {
                        let rel = f.file_path.strip_prefix(&result.project_path).unwrap_or(&f.file_path).display().to_string();
                        by_file.entry(rel).or_default().push(FileEntry { line: f.line_number, redacted: f.redacted_match.clone() });
                    }
                    for (file, entries) in &by_file {
                        if entries.len() == 1 {
                            let e = &entries[0];
                            let loc = if e.line > 0 { format!(":{}", e.line) } else { String::new() };
                            println!("        \x1b[90m{}{}\x1b[0m", file, loc);
                            println!("          \x1b[90m-- {}\x1b[0m", e.redacted);
                            let _ = writeln!(report, "        {}{}", file, loc);
                            let _ = writeln!(report, "          -- {}", e.redacted);
                        } else {
                            println!("        \x1b[90m{}\x1b[0m", file);
                            let _ = writeln!(report, "        {}", file);
                            for e in entries {
                                let loc = if e.line > 0 { format!(":{}", e.line) } else { String::new() };
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
        if total_c > 0 { print!("\x1b[31m{} critical\x1b[0m  ", total_c); let _ = write!(counts, "{} critical  ", total_c); }
        if total_w > 0 { print!("\x1b[33m{} warnings\x1b[0m  ", total_w); let _ = write!(counts, "{} warnings  ", total_w); }
        if total_i > 0 { print!("\x1b[90m{} info\x1b[0m", total_i); let _ = write!(counts, "{} info", total_i); }
        println!("\n");
        let _ = writeln!(report, "  {}\n", counts.trim());
    }

    // ── Git history ──
    if !history.is_empty() {
        println!("  \x1b[1m--- Git History (past commits) ---\x1b[0m\n");
        let _ = writeln!(report, "--- Git History (past commits) ---\n");
        for hf in &history {
            let rel_path = hf.finding.file_path.strip_prefix(&hf.finding.project_path)
                .unwrap_or(&hf.finding.file_path);
            println!("    \x1b[31m!!\x1b[0m  {} — {} \x1b[90m({})\x1b[0m",
                rel_path.display(), hf.finding.description, hf.finding.redacted_match);
            println!("        \x1b[90mcommit {} by {} on {} — {}\x1b[0m",
                hf.commit_sha, hf.author, hf.date, hf.commit_msg);
            let _ = writeln!(report, "    !!  {} — {} ({})", rel_path.display(), hf.finding.description, hf.finding.redacted_match);
            let _ = writeln!(report, "        commit {} by {} on {} — {}", hf.commit_sha, hf.author, hf.date, hf.commit_msg);
        }
        println!("\n  \x1b[31m{} secrets found in git history\x1b[0m", history.len());
        println!("  \x1b[90mThese may still be in the repo even if files were deleted.\x1b[0m");
        println!("  \x1b[90mConsider rotating these credentials.\x1b[0m\n");
        let _ = writeln!(report, "\n  {} secrets in git history — consider rotating credentials.\n", history.len());
    }

    // ── OWASP code patterns ──
    if !patterns.is_empty() {
        println!("  \x1b[1m--- Code Patterns (OWASP Top 10:2025) ---\x1b[0m");
        println!("  \x1b[90mRef: owasp.org/Top10 — categories shown per finding\x1b[0m");
        println!("  \x1b[90mA01 Access Control · A03 Injection · A04 Crypto · A05 Misconfig · A08 Integrity\x1b[0m\n");

        let _ = writeln!(report, "--- Code Patterns (OWASP Top 10:2025 — owasp.org/Top10) ---");
        let _ = writeln!(report, "A01 Broken Access Control · A03 Injection (SQL/Cmd/XSS) · A04 Cryptographic Failures");
        let _ = writeln!(report, "A05 Security Misconfiguration · A08 Software & Data Integrity Failures\n");

        // Group by issue type
        struct IssueGroup { cat: String, severity: scanner::code_patterns::PatternSeverity, suggestion: String,
            files: std::collections::BTreeMap<String, Vec<usize>> }
        let mut by_issue: Vec<(String, IssueGroup)> = Vec::new();

        for pf in &patterns {
            let rel = pf.file_path.strip_prefix(&scan_path).unwrap_or(&pf.file_path).display().to_string();
            if let Some((_, group)) = by_issue.iter_mut().find(|(d, _)| d == &pf.description) {
                group.files.entry(rel).or_default().push(pf.line_number);
            } else {
                let mut files = std::collections::BTreeMap::new();
                files.insert(rel, vec![pf.line_number]);
                by_issue.push((pf.description.clone(), IssueGroup {
                    cat: format!("{}", pf.category), severity: pf.severity,
                    suggestion: pf.suggestion.clone(), files,
                }));
            }
        }

        for (desc, group) in &by_issue {
            let (icon, color) = match group.severity {
                scanner::code_patterns::PatternSeverity::High => ("!!", "31"),
                scanner::code_patterns::PatternSeverity::Medium => ("! ", "33"),
                scanner::code_patterns::PatternSeverity::Low => ("i ", "90"),
            };
            let total_hits: usize = group.files.values().map(|v| v.len()).sum();
            println!("    \x1b[36m[{}]\x1b[0m \x1b[{}m{}\x1b[0m {} \x1b[90m({} hits in {} files)\x1b[0m",
                group.cat, color, icon, desc, total_hits, group.files.len());
            println!("        \x1b[90m-> {}\x1b[0m", group.suggestion);
            let _ = writeln!(report, "    [{}] {} {} ({} hits in {} files)", group.cat, icon, desc, total_hits, group.files.len());
            let _ = writeln!(report, "        -> {}", group.suggestion);

            if group.files.len() <= 8 {
                for (file, lines) in &group.files {
                    let lines_str: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
                    println!("        \x1b[90m{} [{}]\x1b[0m", file, lines_str.join(", "));
                    let _ = writeln!(report, "        {} [{}]", file, lines_str.join(", "));
                }
            } else {
                let mut by_dir: std::collections::BTreeMap<String, Vec<(&String, &Vec<usize>)>> = std::collections::BTreeMap::new();
                for (file, lines) in &group.files {
                    let dir = std::path::Path::new(file).parent()
                        .map(|p| {
                            let s = p.display().to_string();
                            let parts: Vec<&str> = s.split('/').collect();
                            let depth = parts.len().min(3);
                            parts[..depth].join("/")
                        })
                        .unwrap_or_else(|| ".".into());
                    by_dir.entry(dir).or_default().push((file, lines));
                }

                for (dir, files_in_dir) in &by_dir {
                    let dir_hits: usize = files_in_dir.iter().map(|(_, l)| l.len()).sum();
                    println!("\n        \x1b[33m{}/\x1b[0m \x1b[90m({} files, {} hits)\x1b[0m",
                        dir, files_in_dir.len(), dir_hits);
                    let _ = writeln!(report, "\n        {}/ ({} files, {} hits)", dir, files_in_dir.len(), dir_hits);
                    for (file, lines) in files_in_dir {
                        let short = file.strip_prefix(&format!("{}/", dir)).unwrap_or(file);
                        let lines_str: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
                        println!("          \x1b[90m{} [{}]\x1b[0m", short, lines_str.join(", "));
                        let _ = writeln!(report, "          {} [{}]", short, lines_str.join(", "));
                    }
                }
            }
            println!();
            let _ = writeln!(report);
        }

        let high = patterns.iter().filter(|p| p.severity == scanner::code_patterns::PatternSeverity::High).count();
        let med = patterns.iter().filter(|p| p.severity == scanner::code_patterns::PatternSeverity::Medium).count();
        let low = patterns.iter().filter(|p| p.severity == scanner::code_patterns::PatternSeverity::Low).count();
        print!("  ");
        let mut counts = String::new();
        if high > 0 { print!("\x1b[31m{} high\x1b[0m  ", high); let _ = write!(counts, "{} high  ", high); }
        if med > 0 { print!("\x1b[33m{} medium\x1b[0m  ", med); let _ = write!(counts, "{} medium  ", med); }
        if low > 0 { print!("\x1b[90m{} low\x1b[0m", low); let _ = write!(counts, "{} low", low); }
        println!();
        let _ = writeln!(report, "  {}", counts.trim());
    }

    // ── Summary ──
    println!("\n  =================================");
    println!("  SPARK Audit Summary");
    let _ = writeln!(report, "\n=================================\nSPARK Audit Summary");
    if !results.is_empty() {
        let total: usize = results.iter().map(|r| r.findings.len()).sum();
        println!("    Secrets:      {} findings", total);
        let _ = writeln!(report, "  Secrets:      {} findings", total);
    }
    if !history.is_empty() {
        println!("    Git History:  {} findings", history.len());
        let _ = writeln!(report, "  Git History:  {} findings", history.len());
    }
    if !patterns.is_empty() {
        println!("    Code (OWASP): {} findings", patterns.len());
        let _ = writeln!(report, "  Code (OWASP): {} findings", patterns.len());
    }
    println!();

    // ── Ignore tip ──
    if !scan_path.join(".sparkauditignore").exists() && has_findings {
        println!("  \x1b[90mTip: spark audit --init to create .sparkauditignore\x1b[0m");
        println!("  \x1b[90m     Suppress known findings (works like .gitignore)\x1b[0m\n");
    }

    // ── Save to file ──
    if let Some(out_path) = output_file {
        let header = format!("SPARK Security Audit Report\nPath: {}\nDate: {}\n\n",
            scan_path.display(),
            chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"));
        match std::fs::write(&out_path, format!("{}{}", header, report)) {
            Ok(_) => println!("  Report saved to: {}", out_path.display()),
            Err(e) => eprintln!("  Failed to save report: {}", e),
        }
    }
}

fn create_ignore_file(scan_path: &std::path::Path) {
    let ignore_path = scan_path.join(".sparkauditignore");
    if ignore_path.exists() {
        println!("  .sparkauditignore already exists at {}", ignore_path.display());
        return;
    }

    let mut lines = vec![
        "# .sparkauditignore".to_string(),
        "# Paths listed here will be skipped during spark audit.".to_string(),
        "# Works like .gitignore: one path per line, relative to project root.".to_string(),
        "#".to_string(),
        "# Only uncomment a path AFTER reviewing its audit findings and".to_string(),
        "# confirming they are safe or expected (e.g. test fixtures, docs examples).".to_string(),
        "# Each uncommented path means: \"I reviewed this and it's not a risk.\"".to_string(),
        "".to_string(),
    ];

    // Detect project structure and suggest paths (all commented)
    lines.push("# ── Test files (test tokens and fixtures are expected) ──".to_string());
    for dir in &["src/tests/", "tests/", "test/", "__tests__/", "spec/"] {
        if scan_path.join(dir.trim_end_matches('/')).exists() {
            lines.push(format!("# {}  # detected in project", dir));
        }
    }

    lines.push("".to_string());
    lines.push("# ── Documentation (example credentials in guides) ──".to_string());
    if scan_path.join("docs").exists() {
        lines.push("# docs/  # detected in project".to_string());
    }

    lines.push("".to_string());
    lines.push("# ── Seed and fixture data ──".to_string());
    for file in &["src/seed.ts", "src/seed.js", "prisma/seed.ts", "db/seeds/"] {
        if scan_path.join(file.trim_end_matches('/')).exists() {
            lines.push(format!("# {}  # detected in project", file));
        }
    }

    lines.push("".to_string());
    lines.push("# ── Scripts (install/setup may need shell commands) ──".to_string());
    if scan_path.join("scripts").exists() {
        lines.push("# scripts/  # detected in project".to_string());
    }

    lines.push("".to_string());
    lines.push("# ── Other paths to consider ──".to_string());
    lines.push("# archived/".to_string());
    lines.push("# dist/".to_string());
    lines.push("# build/".to_string());

    let content = lines.join("\n") + "\n";
    match std::fs::write(&ignore_path, content) {
        Ok(_) => {
            println!("  Created .sparkauditignore");
            println!("  Edit to add/remove paths, then run spark audit\n");
            // Show what was generated
            let active: Vec<&String> = lines.iter()
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .collect();
            if !active.is_empty() {
                println!("  Auto-detected paths to ignore:");
                for p in &active {
                    println!("    {}", p);
                }
            }
        }
        Err(e) => eprintln!("  Failed to create .sparkauditignore: {}", e),
    }
}
