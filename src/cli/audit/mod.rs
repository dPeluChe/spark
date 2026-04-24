//! Security audit CLI command — secrets, git history, OWASP code patterns, deps.

mod deps;
mod history;
mod ignore;
mod patterns;
mod secrets;

pub use deps::cmd_audit_deps;

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
        ignore::create(&scan_path);
        return;
    }

    let phases = if skip_deps { 3 } else { 4 };
    println!("  SPARK Security Audit");
    if skip_deps {
        println!("  Everything runs locally — nothing leaves your machine.\n");
    } else {
        println!("  Dependency check queries osv.dev (Google OSV, no auth required).\n");
    }

    let results = run_secrets_phase(&scan_path, phases);
    let history = run_history_phase(&scan_path, phases);
    let patterns_found = run_patterns_phase(&scan_path, phases);
    let (dep_result, npm_audit_json) = run_deps_phase(&scan_path, phases, skip_deps);
    println!();

    let has_dep_findings = dep_result
        .as_ref()
        .map(|r| !r.vulnerabilities.is_empty())
        .unwrap_or(false)
        || npm_audit_json.is_some();
    let has_findings = !results.is_empty()
        || !history.is_empty()
        || !patterns_found.is_empty()
        || has_dep_findings;
    if !has_findings {
        println!("  \x1b[32mNo security findings detected.\x1b[0m");
        return;
    }

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

fn run_secrets_phase(
    scan_path: &std::path::Path,
    phases: u8,
) -> Vec<scanner::secret_scanner::AuditResult> {
    eprint!("  [1/{}] Secrets scan", phases);
    let results = scanner::secret_scanner::scan_directory_with_progress(
        scan_path,
        Some(&|count| {
            if count % 50 == 0 {
                eprint!(".");
            }
        }),
    );
    eprintln!(" done");
    results
}

fn run_history_phase(
    scan_path: &std::path::Path,
    phases: u8,
) -> Vec<scanner::history_scanner::HistoryFinding> {
    eprint!("  [2/{}] Git history scan", phases);
    if !scan_path.join(".git").exists() {
        eprintln!(".. skipped (no .git)");
        return Vec::new();
    }
    let h = scanner::history_scanner::scan_history(scan_path);
    eprintln!(".. {} findings", h.len());
    h
}

fn run_patterns_phase(
    scan_path: &std::path::Path,
    phases: u8,
) -> Vec<scanner::code_patterns::PatternFinding> {
    eprint!("  [3/{}] Code patterns scan", phases);
    let patterns = scanner::code_patterns::scan_code_patterns(scan_path);
    eprintln!(".. {} findings", patterns.len());
    patterns
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
