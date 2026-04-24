//! Create `.sparkauditignore` scaffold (`spark audit --init`).

use std::path::Path;

pub(super) fn create(scan_path: &Path) {
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

    lines.push("# ── Test files (test tokens and fixtures are expected) ──".to_string());
    for dir in &["src/tests/", "tests/", "test/", "__tests__/", "spec/"] {
        if scan_path.join(dir.trim_end_matches('/')).exists() {
            lines.push(format!("# {}  # detected in project", dir));
        }
    }

    lines.push(String::new());
    lines.push("# ── Documentation (example credentials in guides) ──".to_string());
    if scan_path.join("docs").exists() {
        lines.push("# docs/  # detected in project".to_string());
    }

    lines.push(String::new());
    lines.push("# ── Seed and fixture data ──".to_string());
    for file in &["src/seed.ts", "src/seed.js", "prisma/seed.ts", "db/seeds/"] {
        if scan_path.join(file.trim_end_matches('/')).exists() {
            lines.push(format!("# {}  # detected in project", file));
        }
    }

    lines.push(String::new());
    lines.push("# ── Scripts (install/setup may need shell commands) ──".to_string());
    if scan_path.join("scripts").exists() {
        lines.push("# scripts/  # detected in project".to_string());
    }

    lines.push(String::new());
    lines.push("# ── Other paths to consider ──".to_string());
    lines.push("# archived/".to_string());
    lines.push("# dist/".to_string());
    lines.push("# build/".to_string());

    let content = lines.join("\n") + "\n";
    match std::fs::write(&ignore_path, content) {
        Ok(_) => {
            println!("  Created .sparkauditignore");
            println!("  Edit to add/remove paths, then run spark audit\n");
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
