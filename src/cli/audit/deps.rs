//! Dependency vulnerability section: render in combined audit + standalone `spark audit --deps`.

use crate::scanner;
use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

/// Render the OSV.dev results inline in the main audit report.
pub(super) fn render(
    report: &mut String,
    dep_result: Option<&scanner::dep_scanner::DepScanResult>,
) {
    let dep = match dep_result {
        Some(d) if !d.vulnerabilities.is_empty() => d,
        _ => return,
    };

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
        let (icon, color) = severity_marks(&first.severity);
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

    render_severity_summary(report, &dep.vulnerabilities);
}

/// Render npm audit JSON (from `npm audit --json`) in the main audit report.
pub(super) fn render_npm(report: &mut String, json: &serde_json::Value) {
    println!("  \x1b[1m--- npm audit ---\x1b[0m\n");
    let _ = writeln!(report, "--- npm audit ---\n");
    if let Some(vulns) = json.get("vulnerabilities").and_then(|v| v.as_object()) {
        for (name, info) in vulns {
            render_npm_finding(report, name, info);
        }
    }
    println!("\n  \x1b[90mRun `npm audit fix` to auto-fix where possible.\x1b[0m\n");
    let _ = writeln!(
        report,
        "\n  Run `npm audit fix` to auto-fix where possible.\n"
    );
}

fn render_npm_finding(report: &mut String, name: &str, info: &serde_json::Value) {
    let severity = info
        .get("severity")
        .and_then(|s| s.as_str())
        .unwrap_or("unknown");
    let via = info
        .get("via")
        .and_then(|v| {
            v.as_array().and_then(|arr| {
                arr.first().and_then(|item| {
                    item.as_str().map(|s| s.to_string()).or_else(|| {
                        item.get("title")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string())
                    })
                })
            })
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

fn severity_marks(sev: &str) -> (&'static str, &'static str) {
    match sev.to_uppercase().as_str() {
        "CRITICAL" | "HIGH" => ("!!", "31"),
        "MODERATE" | "MEDIUM" => ("! ", "33"),
        _ => ("i ", "90"),
    }
}

fn render_severity_summary(report: &mut String, vulns: &[scanner::dep_scanner::DepVulnerability]) {
    let crit = vulns
        .iter()
        .filter(|v| v.severity.to_uppercase() == "CRITICAL")
        .count();
    let high = vulns
        .iter()
        .filter(|v| v.severity.to_uppercase() == "HIGH")
        .count();
    let med = vulns
        .iter()
        .filter(|v| matches!(v.severity.to_uppercase().as_str(), "MODERATE" | "MEDIUM"))
        .count();
    let low = vulns
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

/// Standalone `spark audit --deps` command (skips secrets/history/patterns).
pub fn cmd_audit_deps(path: Option<PathBuf>) {
    let scan_path = path.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    println!("  SPARK Dependency Scan");
    println!("  Queries osv.dev (Google OSV) + npm audit if available.\n");

    let deps = scanner::dep_scanner::parse_dependencies(&scan_path);
    if deps.is_empty() {
        println!("  No dependency files found (package.json, requirements.txt, Cargo.toml/lock)");
        return;
    }

    print_dep_counts(&deps);

    eprint!("  [1/2] OSV.dev scan");
    let result = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current()
            .block_on(scanner::dep_scanner::check_vulnerabilities(&deps))
    });
    eprintln!(".. {} vulnerabilities", result.vulnerabilities.len());

    let npm_count = deps.iter().filter(|d| d.ecosystem == "npm").count();
    let npm_audit_output = run_npm_audit(npm_count, &scan_path);
    println!();

    print_osv_findings(&result);
    if let Some(ref json) = npm_audit_output {
        print_npm_findings(json);
    }

    println!("\n  =================================");
    println!("  Dependency Scan Summary");
    println!("    Checked:        {} deps", result.deps_checked);
    println!("    Vulnerabilities: {}", result.vulnerabilities.len());
    if npm_audit_output.is_some() {
        println!("    npm audit:       included");
    }
    println!();
}

fn print_dep_counts(deps: &[scanner::dep_scanner::Dependency]) {
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
}

fn run_npm_audit(npm_count: usize, scan_path: &Path) -> Option<serde_json::Value> {
    if npm_count == 0 || !scan_path.join("package-lock.json").exists() {
        eprintln!("  [2/2] npm audit.. skipped (no package-lock.json)");
        return None;
    }
    eprint!("  [2/2] npm audit");
    match std::process::Command::new("npm")
        .args(["audit", "--json"])
        .current_dir(scan_path)
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
}

fn print_osv_findings(result: &scanner::dep_scanner::DepScanResult) {
    if result.vulnerabilities.is_empty() {
        return;
    }
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
        let (icon, color) = severity_marks(&first.severity);
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

fn print_npm_findings(json: &serde_json::Value) {
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
                    v.as_array().and_then(|arr| {
                        arr.first().and_then(|item| {
                            item.as_str().map(|s| s.to_string()).or_else(|| {
                                item.get("title")
                                    .and_then(|t| t.as_str())
                                    .map(|s| s.to_string())
                            })
                        })
                    })
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
