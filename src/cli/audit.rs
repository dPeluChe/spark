//! Security audit CLI command — secrets, git history, and OWASP code patterns.

use super::shorten_path;
use crate::scanner;
use std::fmt::Write as FmtWrite;
use std::path::PathBuf;

pub fn cmd_audit(
    path: Option<PathBuf>,
    output_file: Option<PathBuf>,
    init_ignore: bool,
    skip_deps: bool,
) {
    let scan_path = path.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    if init_ignore {
        create_ignore_file(&scan_path);
        return;
    }

    let phases = if skip_deps { 3 } else { 4 };
    println!("  SPARK Security Audit");
    if skip_deps {
        println!("  Everything runs locally — nothing leaves your machine.\n");
    } else {
        println!("  Dependency check queries osv.dev (Google OSV, no auth required).\n");
    }

    // Phase 1: Secret scan (files)
    eprint!("  [1/{}] Secrets scan", phases);
    let results = scanner::secret_scanner::scan_directory_with_progress(
        &scan_path,
        Some(&|count| {
            if count % 50 == 0 {
                eprint!(".");
            }
        }),
    );
    eprintln!(" done");

    // Phase 2: Git history scan
    eprint!("  [2/{}] Git history scan", phases);
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
    eprint!("  [3/{}] Code patterns scan", phases);
    let patterns = scanner::code_patterns::scan_code_patterns(&scan_path);
    eprintln!(".. {} findings", patterns.len());

    // Phase 4: Dependency vulnerabilities (OSV.dev + npm audit)
    let mut npm_audit_json: Option<serde_json::Value> = None;
    let dep_result = if !skip_deps {
        eprint!("  [4/{}] Dependency scan", phases);
        let deps = scanner::dep_scanner::parse_dependencies(&scan_path);
        if deps.is_empty() {
            eprintln!(".. no dependency files found");
            None
        } else {
            eprint!(" ({} deps)", deps.len());
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current()
                    .block_on(scanner::dep_scanner::check_vulnerabilities(&deps))
            });
            eprintln!(".. {} vulnerabilities", result.vulnerabilities.len());

            // npm audit if project has package-lock.json
            let has_npm = deps.iter().any(|d| d.ecosystem == "npm")
                && scan_path.join("package-lock.json").exists();
            if has_npm {
                eprint!("        npm audit");
                if let Ok(o) = std::process::Command::new("npm")
                    .args(["audit", "--json"])
                    .current_dir(&scan_path)
                    .output()
                {
                    let json: serde_json::Value =
                        serde_json::from_slice(&o.stdout).unwrap_or_default();
                    let total = json
                        .get("metadata")
                        .and_then(|m| m.get("vulnerabilities"))
                        .and_then(|v| v.as_object())
                        .map(|obj| obj.values().filter_map(|v| v.as_u64()).sum::<u64>())
                        .unwrap_or(0);
                    eprintln!(".. {} issues", total);
                    if total > 0 {
                        npm_audit_json = Some(json);
                    }
                } else {
                    eprintln!(".. skipped (npm not found)");
                }
            }

            Some(result)
        }
    } else {
        None
    };
    println!();

    let has_dep_findings = dep_result
        .as_ref()
        .map(|r| !r.vulnerabilities.is_empty())
        .unwrap_or(false)
        || npm_audit_json.is_some();
    let has_findings =
        !results.is_empty() || !history.is_empty() || !patterns.is_empty() || has_dep_findings;
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
            println!(
                "  \x1b[1m{}\x1b[0m  \x1b[90m{}\x1b[0m",
                result.project_name, short
            );
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

    // ── Git history ──
    if !history.is_empty() {
        println!("  \x1b[1m--- Git History (past commits) ---\x1b[0m\n");
        let _ = writeln!(report, "--- Git History (past commits) ---\n");
        for hf in &history {
            let rel_path = hf
                .finding
                .file_path
                .strip_prefix(&hf.finding.project_path)
                .unwrap_or(&hf.finding.file_path);
            println!(
                "    \x1b[31m!!\x1b[0m  {} — {} \x1b[90m({})\x1b[0m",
                rel_path.display(),
                hf.finding.description,
                hf.finding.redacted_match
            );
            println!(
                "        \x1b[90mcommit {} by {} on {} — {}\x1b[0m",
                hf.commit_sha, hf.author, hf.date, hf.commit_msg
            );
            let _ = writeln!(
                report,
                "    !!  {} — {} ({})",
                rel_path.display(),
                hf.finding.description,
                hf.finding.redacted_match
            );
            let _ = writeln!(
                report,
                "        commit {} by {} on {} — {}",
                hf.commit_sha, hf.author, hf.date, hf.commit_msg
            );
        }
        println!(
            "\n  \x1b[31m{} secrets found in git history\x1b[0m",
            history.len()
        );
        println!("  \x1b[90mThese may still be in the repo even if files were deleted.\x1b[0m");
        println!("  \x1b[90mConsider rotating these credentials.\x1b[0m\n");
        let _ = writeln!(
            report,
            "\n  {} secrets in git history — consider rotating credentials.\n",
            history.len()
        );
    }

    // ── OWASP code patterns ──
    if !patterns.is_empty() {
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
        let mut by_issue: Vec<(String, IssueGroup)> = Vec::new();

        for pf in &patterns {
            let rel = pf
                .file_path
                .strip_prefix(&scan_path)
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
            } else {
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
            println!();
            let _ = writeln!(report);
        }

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

    // ── Dependency vulnerabilities ──
    if let Some(ref dep) = dep_result {
        if !dep.vulnerabilities.is_empty() {
            println!("  \x1b[1m--- Dependencies (OSV.dev) ---\x1b[0m");
            println!(
                "  \x1b[90mSource: osv.dev — {} deps checked\x1b[0m\n",
                dep.deps_checked
            );
            let _ = writeln!(
                report,
                "--- Dependencies (OSV.dev) — {} deps checked ---\n",
                dep.deps_checked
            );

            // Group by dep name
            let mut by_dep: std::collections::BTreeMap<
                String,
                Vec<&scanner::dep_scanner::DepVulnerability>,
            > = std::collections::BTreeMap::new();
            for v in &dep.vulnerabilities {
                by_dep
                    .entry(format!("{} ({})", v.dep_name, v.ecosystem))
                    .or_default()
                    .push(v);
            }

            for (dep_key, vulns) in &by_dep {
                let first = vulns[0];
                let (icon, color) = match first.severity.to_uppercase().as_str() {
                    "CRITICAL" => ("!!", "31"),
                    "HIGH" => ("!!", "31"),
                    "MODERATE" | "MEDIUM" => ("! ", "33"),
                    _ => ("i ", "90"),
                };
                println!(
                    "    \x1b[{}m{}\x1b[0m  {} \x1b[90mv{} — from {}\x1b[0m",
                    color, icon, dep_key, first.dep_version, first.source_file
                );
                let _ = writeln!(
                    report,
                    "    {}  {} v{} — from {}",
                    icon, dep_key, first.dep_version, first.source_file
                );

                for v in vulns {
                    let fix = v.fixed_version.as_deref().unwrap_or("no fix yet");
                    println!(
                        "        \x1b[90m-- {} [{}] {}\x1b[0m",
                        v.id, v.severity, v.summary
                    );
                    println!("           \x1b[90m-> upgrade to {}\x1b[0m", fix);
                    let _ = writeln!(report, "        -- {} [{}] {}", v.id, v.severity, v.summary);
                    let _ = writeln!(report, "           -> upgrade to {}", fix);
                }
                println!();
                let _ = writeln!(report);
            }

            let crit = dep
                .vulnerabilities
                .iter()
                .filter(|v| matches!(v.severity.to_uppercase().as_str(), "CRITICAL"))
                .count();
            let high = dep
                .vulnerabilities
                .iter()
                .filter(|v| matches!(v.severity.to_uppercase().as_str(), "HIGH"))
                .count();
            let med = dep
                .vulnerabilities
                .iter()
                .filter(|v| matches!(v.severity.to_uppercase().as_str(), "MODERATE" | "MEDIUM"))
                .count();
            let low = dep
                .vulnerabilities
                .iter()
                .filter(|v| {
                    !matches!(
                        v.severity.to_uppercase().as_str(),
                        "CRITICAL" | "HIGH" | "MODERATE" | "MEDIUM"
                    )
                })
                .count();
            print!("  ");
            let mut counts = String::new();
            if crit > 0 {
                print!("\x1b[31m{} critical\x1b[0m  ", crit);
                let _ = write!(counts, "{} critical  ", crit);
            }
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
    }

    // ── npm audit findings ──
    if let Some(ref json) = npm_audit_json {
        println!("  \x1b[1m--- npm audit ---\x1b[0m\n");
        let _ = writeln!(report, "--- npm audit ---\n");
        if let Some(vulns) = json.get("vulnerabilities").and_then(|v| v.as_object()) {
            for (name, info) in vulns {
                let severity = info
                    .get("severity")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown");
                let via = info
                    .get("via")
                    .and_then(|v| {
                        if let Some(arr) = v.as_array() {
                            arr.first().and_then(|item| {
                                item.as_str().map(|s| s.to_string()).or_else(|| {
                                    item.get("title")
                                        .and_then(|t| t.as_str())
                                        .map(|s| s.to_string())
                                })
                            })
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                let fix_available = info
                    .get("fixAvailable")
                    .and_then(|f| f.as_bool())
                    .unwrap_or(false);
                let (icon, color) = match severity {
                    "critical" | "high" => ("!!", "31"),
                    "moderate" => ("! ", "33"),
                    _ => ("i ", "90"),
                };
                let via_str = if !via.is_empty() {
                    format!(" — {}", via)
                } else {
                    String::new()
                };
                println!(
                    "    \x1b[{}m{}\x1b[0m  {} \x1b[90m[{}]{}\x1b[0m",
                    color, icon, name, severity, via_str
                );
                let _ = writeln!(report, "    {}  {} [{}]{}", icon, name, severity, via_str);
                if fix_available {
                    println!("        \x1b[90m-> npm audit fix\x1b[0m");
                    let _ = writeln!(report, "        -> npm audit fix");
                }
            }
        }
        println!("\n  \x1b[90mRun `npm audit fix` to auto-fix where possible.\x1b[0m\n");
        let _ = writeln!(
            report,
            "\n  Run `npm audit fix` to auto-fix where possible.\n"
        );
    }

    // ── Summary ──
    println!("\n  =================================");
    println!("  SPARK Audit Summary");
    let _ = writeln!(
        report,
        "\n=================================\nSPARK Audit Summary"
    );
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
    if let Some(ref dep) = dep_result {
        println!(
            "    Dependencies: {} deps, {} vulnerabilities",
            dep.deps_checked,
            dep.vulnerabilities.len()
        );
        let _ = writeln!(
            report,
            "  Dependencies: {} deps, {} vulnerabilities",
            dep.deps_checked,
            dep.vulnerabilities.len()
        );
    }
    if npm_audit_json.is_some() {
        println!("    npm audit:    included");
        let _ = writeln!(report, "  npm audit:    included");
    }
    println!();

    // ── Ignore tip ──
    if !scan_path.join(".sparkauditignore").exists() && has_findings {
        println!("  \x1b[90mTip: spark audit --init to create .sparkauditignore\x1b[0m");
        println!("  \x1b[90m     Suppress known findings (works like .gitignore)\x1b[0m\n");
    }

    // ── Save to file ──
    if let Some(out_path) = output_file {
        let header = format!(
            "SPARK Security Audit Report\nPath: {}\nDate: {}\n\n",
            scan_path.display(),
            chrono::Utc::now().format("%Y-%m-%d %H:%M UTC")
        );
        match std::fs::write(&out_path, format!("{}{}", header, report)) {
            Ok(_) => println!("  Report saved to: {}", out_path.display()),
            Err(e) => eprintln!("  Failed to save report: {}", e),
        }
    }
}

/// Dependency-only scan mode
pub fn cmd_audit_deps(path: Option<PathBuf>) {
    let scan_path = path.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    println!("  SPARK Dependency Scan");
    println!("  Queries osv.dev (Google OSV) + npm audit if available.\n");

    // Parse dependencies
    let deps = scanner::dep_scanner::parse_dependencies(&scan_path);
    if deps.is_empty() {
        println!("  No dependency files found (package.json, requirements.txt, Cargo.toml/lock)");
        return;
    }

    let npm_count = deps.iter().filter(|d| d.ecosystem == "npm").count();
    let pypi_count = deps.iter().filter(|d| d.ecosystem == "PyPI").count();
    let cargo_count = deps.iter().filter(|d| d.ecosystem == "crates.io").count();

    println!("  Found {} dependencies:", deps.len());
    if npm_count > 0 {
        println!("    npm:      {}", npm_count);
    }
    if pypi_count > 0 {
        println!("    pip:      {}", pypi_count);
    }
    if cargo_count > 0 {
        println!("    cargo:    {}", cargo_count);
    }
    println!();

    // OSV.dev scan
    eprint!("  [1/2] OSV.dev scan");
    let result = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(scanner::dep_scanner::check_vulnerabilities(&deps))
    });
    eprintln!(".. {} vulnerabilities", result.vulnerabilities.len());

    // npm audit (if npm project and npm available)
    let npm_audit_output = if npm_count > 0 && scan_path.join("package-lock.json").exists() {
        eprint!("  [2/2] npm audit");
        match std::process::Command::new("npm")
            .args(["audit", "--json"])
            .current_dir(&scan_path)
            .output()
        {
            Ok(o) => {
                let json: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap_or_default();
                let total = json
                    .get("metadata")
                    .and_then(|m| m.get("vulnerabilities"))
                    .and_then(|v| v.as_object())
                    .map(|obj| obj.values().filter_map(|v| v.as_u64()).sum::<u64>())
                    .unwrap_or(0);
                eprintln!(".. {} vulnerabilities", total);
                if total > 0 {
                    Some(json)
                } else {
                    None
                }
            }
            Err(_) => {
                eprintln!(".. skipped (npm not found)");
                None
            }
        }
    } else {
        eprintln!("  [2/2] npm audit.. skipped (no package-lock.json)");
        None
    };
    println!();

    // Print OSV results
    if !result.vulnerabilities.is_empty() {
        println!("  \x1b[1m--- OSV.dev Findings ---\x1b[0m\n");
        let mut by_dep: std::collections::BTreeMap<
            String,
            Vec<&scanner::dep_scanner::DepVulnerability>,
        > = std::collections::BTreeMap::new();
        for v in &result.vulnerabilities {
            by_dep
                .entry(format!("{} ({})", v.dep_name, v.ecosystem))
                .or_default()
                .push(v);
        }
        for (dep_key, vulns) in &by_dep {
            let first = vulns[0];
            let (icon, color) = match first.severity.to_uppercase().as_str() {
                "CRITICAL" | "HIGH" => ("!!", "31"),
                "MODERATE" | "MEDIUM" => ("! ", "33"),
                _ => ("i ", "90"),
            };
            println!(
                "    \x1b[{}m{}\x1b[0m  {} v{} \x1b[90m({})\x1b[0m",
                color, icon, dep_key, first.dep_version, first.source_file
            );
            for v in vulns {
                let fix = v.fixed_version.as_deref().unwrap_or("no fix yet");
                println!(
                    "        \x1b[90m-- {} [{}] {}\x1b[0m",
                    v.id, v.severity, v.summary
                );
                println!("           \x1b[90m-> upgrade to {}\x1b[0m", fix);
            }
            println!();
        }
    }

    // Print npm audit summary
    if let Some(ref json) = npm_audit_output {
        println!("  \x1b[1m--- npm audit Findings ---\x1b[0m\n");
        if let Some(vulns) = json.get("vulnerabilities").and_then(|v| v.as_object()) {
            for (name, info) in vulns {
                let severity = info
                    .get("severity")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown");
                let via = info
                    .get("via")
                    .and_then(|v| {
                        if let Some(arr) = v.as_array() {
                            arr.first().and_then(|item| {
                                item.as_str().map(|s| s.to_string()).or_else(|| {
                                    item.get("title")
                                        .and_then(|t| t.as_str())
                                        .map(|s| s.to_string())
                                })
                            })
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default();
                let fix_available = info
                    .get("fixAvailable")
                    .and_then(|f| f.as_bool())
                    .unwrap_or(false);
                let (icon, color) = match severity {
                    "critical" | "high" => ("!!", "31"),
                    "moderate" => ("! ", "33"),
                    _ => ("i ", "90"),
                };
                println!(
                    "    \x1b[{}m{}\x1b[0m  {} \x1b[90m[{}]{}\x1b[0m",
                    color,
                    icon,
                    name,
                    severity,
                    if !via.is_empty() {
                        format!(" — {}", via)
                    } else {
                        String::new()
                    }
                );
                if fix_available {
                    println!("        \x1b[90m-> fix available: npm audit fix\x1b[0m");
                }
            }
        }
        println!();
        println!("  \x1b[90mRun `npm audit fix` to auto-fix where possible.\x1b[0m");
    }

    // Summary
    println!("\n  =================================");
    println!("  Dependency Scan Summary");
    println!("    Checked:        {} deps", result.deps_checked);
    println!("    Vulnerabilities: {}", result.vulnerabilities.len());
    if npm_audit_output.is_some() {
        println!("    npm audit:       included");
    }
    println!();
}

fn create_ignore_file(scan_path: &std::path::Path) {
    let ignore_path = scan_path.join(".sparkauditignore");
    if ignore_path.exists() {
        println!(
            "  .sparkauditignore already exists at {}",
            ignore_path.display()
        );
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
            let active: Vec<&String> = lines
                .iter()
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
