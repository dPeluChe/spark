//! Security audit CLI command — secrets, git history, OWASP code patterns, deps.

mod deps;
mod history;
mod ignore;
mod patterns;
mod secrets;

pub use deps::cmd_audit_deps;

use crate::cli::json;
use crate::scanner;
use std::fmt::Write as FmtWrite;
use std::path::PathBuf;

pub fn cmd_audit(
    path: Option<PathBuf>,
    output_file: Option<PathBuf>,
    init_ignore: bool,
    skip_deps: bool,
    json_out: bool,
) {
    let scan_path = path.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    if init_ignore {
        ignore::create(&scan_path);
        return;
    }

    let phases = if skip_deps { 3 } else { 4 };
    if !json_out {
        println!("  SPARK Security Audit");
        if skip_deps {
            println!("  Everything runs locally — nothing leaves your machine.\n");
        } else {
            println!("  Dependency check queries osv.dev (Google OSV, no auth required).\n");
        }
    }

    // Phases 1-3 are independent local scans — run them in parallel. Phase 4
    // (network + npm audit) runs on the main thread, which holds the tokio
    // context that its block_in_place needs.
    let (results, history, patterns_found, dep_result, npm_audit_json) = std::thread::scope(|s| {
        eprintln!(
            "  [1-3/{}] Secrets + git history + code patterns (parallel)",
            phases
        );
        let secrets_h = s.spawn(|| scanner::secret_scanner::scan_directory(&scan_path));
        let history_h = s.spawn(|| {
            if scan_path.join(".git").exists() {
                scanner::history_scanner::scan_history(&scan_path)
            } else {
                Vec::new()
            }
        });
        let patterns_h = s.spawn(|| scanner::code_patterns::scan_code_patterns(&scan_path));
        let (dep_result, npm_audit_json) = run_deps_phase(&scan_path, phases, skip_deps);
        (
            secrets_h.join().unwrap_or_default(),
            history_h.join().unwrap_or_default(),
            patterns_h.join().unwrap_or_default(),
            dep_result,
            npm_audit_json,
        )
    });

    let secrets_total: usize = results.iter().map(|r| r.findings.len()).sum();
    eprintln!("    Secrets:       {} findings", secrets_total);
    eprintln!("    Git history:   {} findings", history.len());
    eprintln!("    Code (OWASP):  {} findings", patterns_found.len());
    eprintln!();

    let has_dep_findings = dep_result
        .as_ref()
        .map(|r| !r.vulnerabilities.is_empty())
        .unwrap_or(false)
        || npm_audit_json.is_some();
    let has_findings = !results.is_empty()
        || !history.is_empty()
        || !patterns_found.is_empty()
        || has_dep_findings;

    persist_last_audit(
        &scan_path,
        secrets_total + history.len() + patterns_found.len(),
    );

    if json_out {
        let payload = build_audit_json(
            &scan_path,
            &results,
            &history,
            &patterns_found,
            dep_result.as_ref(),
            npm_audit_json.is_some(),
        );
        if let Some(out) = &output_file {
            let _ = std::fs::write(
                out,
                serde_json::to_string_pretty(&payload).unwrap_or_default(),
            );
        }
        json::print(&payload);
    }

    if !has_findings {
        if !json_out {
            println!("  \x1b[32mNo security findings detected.\x1b[0m");
        }
        return;
    }

    if !json_out {
        let mut report = String::new();

        secrets::render(&mut report, &results);
        history::render(&mut report, &history);
        patterns::render(&mut report, &patterns_found, &scan_path);
        deps::render(&mut report, dep_result.as_ref());
        if let Some(ref json) = npm_audit_json {
            deps::render_npm(&mut report, json);
        }

        render_summary(
            &mut report,
            &results,
            &history,
            &patterns_found,
            dep_result.as_ref(),
            npm_audit_json.is_some(),
        );

        render_ignore_tip(&scan_path, has_findings);
        save_report_if_requested(output_file, &scan_path, &report);
    }

    // CI gate: findings exit non-zero (see ROADMAP.md Phase 0)
    std::process::exit(1);
}

/// Persist a summary so `spark report` can show the last audit at a glance.
fn persist_last_audit(scan_path: &std::path::Path, total: usize) {
    let Some(dir) = dirs::config_dir().map(|d| d.join("spark")) else {
        return;
    };
    let _ = std::fs::create_dir_all(&dir);
    let payload = json::ReportSecurity {
        generated_at: json::now_iso(),
        path: scan_path.display().to_string(),
        total,
    };
    let _ = std::fs::write(
        dir.join("last_audit.json"),
        serde_json::to_string(&payload).unwrap_or_default(),
    );
}

/// Build the `--json` contract from the phase outputs (see ROADMAP.md).
fn build_audit_json(
    scan_path: &std::path::Path,
    results: &[scanner::secret_scanner::AuditResult],
    history: &[scanner::history_scanner::HistoryFinding],
    patterns: &[scanner::code_patterns::PatternFinding],
    dep_result: Option<&scanner::dep_scanner::DepScanResult>,
    has_npm_audit: bool,
) -> json::AuditJson {
    let secrets_total: usize = results.iter().map(|r| r.findings.len()).sum();
    let deps_total = dep_result.map(|r| r.vulnerabilities.len()).unwrap_or(0);
    json::AuditJson {
        json_version: json::JSON_VERSION,
        path: scan_path.display().to_string(),
        generated_at: json::now_iso(),
        summary: json::AuditSummaryJson {
            total: secrets_total + history.len() + patterns.len() + deps_total,
            secrets: secrets_total,
            history: history.len(),
            patterns: patterns.len(),
            deps: deps_total,
            npm_audit: has_npm_audit,
        },
        secrets: results.to_vec(),
        history: history.to_vec(),
        patterns: patterns.to_vec(),
        deps: dep_result.cloned(),
    }
}

fn run_deps_phase(
    scan_path: &std::path::Path,
    phases: u8,
    skip_deps: bool,
) -> (
    Option<scanner::dep_scanner::DepScanResult>,
    Option<serde_json::Value>,
) {
    if skip_deps {
        return (None, None);
    }

    eprint!("  [4/{}] Dependency scan", phases);
    let deps = scanner::dep_scanner::parse_dependencies(scan_path);
    if deps.is_empty() {
        eprintln!(".. no dependency files found");
        return (None, None);
    }

    eprint!(" ({} deps)", deps.len());
    let result = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(scanner::dep_scanner::check_vulnerabilities(&deps))
    });
    eprintln!(".. {} vulnerabilities", result.vulnerabilities.len());

    let has_npm =
        deps.iter().any(|d| d.ecosystem == "npm") && scan_path.join("package-lock.json").exists();
    let npm_audit_json = if has_npm {
        run_npm_audit(scan_path)
    } else {
        None
    };

    (Some(result), npm_audit_json)
}

fn run_npm_audit(scan_path: &std::path::Path) -> Option<serde_json::Value> {
    eprint!("        npm audit");
    let Ok(o) = std::process::Command::new("npm")
        .args(["audit", "--json"])
        .current_dir(scan_path)
        .output()
    else {
        eprintln!(".. skipped (npm not found)");
        return None;
    };
    let json: serde_json::Value = serde_json::from_slice(&o.stdout).unwrap_or_default();
    let total = json
        .get("metadata")
        .and_then(|m| m.get("vulnerabilities"))
        .and_then(|v| v.as_object())
        .map(|obj| obj.values().filter_map(|v| v.as_u64()).sum::<u64>())
        .unwrap_or(0);
    eprintln!(".. {} issues", total);
    if total > 0 {
        Some(json)
    } else {
        None
    }
}

fn render_summary(
    report: &mut String,
    results: &[scanner::secret_scanner::AuditResult],
    history: &[scanner::history_scanner::HistoryFinding],
    patterns: &[scanner::code_patterns::PatternFinding],
    dep_result: Option<&scanner::dep_scanner::DepScanResult>,
    has_npm_audit: bool,
) {
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
    if let Some(dep) = dep_result {
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
    if has_npm_audit {
        println!("    npm audit:    included");
        let _ = writeln!(report, "  npm audit:    included");
    }
    println!();
}

fn render_ignore_tip(scan_path: &std::path::Path, has_findings: bool) {
    if !scan_path.join(".sparkauditignore").exists() && has_findings {
        println!("  \x1b[90mTip: spark audit --init to create .sparkauditignore\x1b[0m");
        println!("  \x1b[90m     Suppress known findings (works like .gitignore)\x1b[0m\n");
    }
}

fn save_report_if_requested(
    output_file: Option<PathBuf>,
    scan_path: &std::path::Path,
    report: &str,
) {
    let Some(out_path) = output_file else { return };
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
