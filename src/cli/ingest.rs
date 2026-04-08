//! Repository ingest CLI — generate LLM-ready context files.

use crate::config;
use crate::scanner::{repo_manager, repo_ingest};
use crate::utils::fs::format_size;
use super::shorten_path;

pub fn cmd_ingest(query: Option<String>, list: bool, compress: bool, config: &config::SparkConfig) {
    if list {
        cmd_ingest_list();
        return;
    }

    // Check repomix availability
    if !repo_ingest::is_repomix_available() {
        eprintln!("  repomix not found. Install with:");
        eprintln!("    npm install -g repomix");
        eprintln!("  or it will be used via npx (requires Node.js)");
        std::process::exit(1);
    }

    let repos = repo_manager::list_managed_repos(&config.repos_root);

    if let Some(q) = query {
        // Ingest specific repo
        let q_lower = q.to_lowercase();
        let found: Vec<_> = repos.iter().filter(|r| {
            r.name.to_lowercase() == q_lower
                || format!("{}/{}", r.owner, r.name).to_lowercase() == q_lower
                || r.name.to_lowercase().contains(&q_lower)
        }).collect();

        if found.is_empty() {
            eprintln!("  No repo matching '{}'", q);
            std::process::exit(1);
        }
        if found.len() > 1 {
            eprintln!("  {} repos match '{}'. Be more specific:\n", found.len(), q);
            for r in &found { eprintln!("    spark ingest {}/{}", r.owner, r.name); }
            std::process::exit(1);
        }

        let repo = found[0];
        generate_one(repo, compress);
    } else {
        // No query — ingest all repos
        println!("  SPARK Ingest — generating LLM context for {} repos\n", repos.len());
        let mut ok = 0;
        let mut skipped = 0;
        let mut errors = 0;

        for (i, repo) in repos.iter().enumerate() {
            eprint!("\r  [{}/{}] {}/{}          ", i + 1, repos.len(), repo.owner, repo.name);

            // Skip if ingest exists and repo hasn't changed
            if let Some(info) = repo_ingest::ingest_info(&repo.host, &repo.owner, &repo.name) {
                if let Some(age) = info.age_secs {
                    if age < 3600 * 24 { // less than 24h old
                        skipped += 1;
                        continue;
                    }
                }
            }

            match repo_ingest::generate_ingest(&repo.path, &repo.host, &repo.owner, &repo.name, compress) {
                Ok(_) => ok += 1,
                Err(_) => errors += 1,
            }
        }
        eprintln!("\r{}\r", " ".repeat(60));

        println!("  {} generated, {} skipped (recent), {} errors", ok, skipped, errors);
    }
}

fn generate_one(repo: &repo_manager::ManagedRepo, compress: bool) {
    let compress_label = if compress { " (compressed)" } else { "" };
    println!("  Generating LLM context for {}/{}{}\n", repo.owner, repo.name, compress_label);

    eprint!("  repomix running");
    match repo_ingest::generate_ingest(&repo.path, &repo.host, &repo.owner, &repo.name, compress) {
        Ok(path) => {
            eprintln!(".. done\n");
            let info = repo_ingest::ingest_info(&repo.host, &repo.owner, &repo.name);
            let size = info.as_ref().map(|i| format_size(i.size)).unwrap_or_default();
            let short = shorten_path(&path.display().to_string());
            println!("  Output: {} ({})", short, size);
            println!("  Use: paste this file into any LLM for full project context");
        }
        Err(e) => {
            eprintln!(".. failed\n");
            eprintln!("  Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_ingest_list() {
    let ingests = repo_ingest::list_ingests();
    if ingests.is_empty() {
        println!("  No ingest files found.");
        println!("  Use: spark ingest <repo> to generate");
        return;
    }

    println!("  \x1b[1m{} ingest files\x1b[0m\n", ingests.len());

    let max_name = ingests.iter()
        .map(|(_, o, n, _)| o.len() + 1 + n.len())
        .max().unwrap_or(20) + 2;

    for (host, owner, name, info) in &ingests {
        let repo_name = format!("{}/{}", owner, name);
        let size = format_size(info.size);
        let age = info.age_display();
        let short = shorten_path(&info.path.display().to_string());
        println!("  {:<width$}  {}  {:>8}  \x1b[90m{}\x1b[0m",
            repo_name, age, size, short, width = max_name);
        let _ = host; // used in path
    }

    println!("\n  \x1b[90mspark ingest <repo>       regenerate\x1b[0m");
    println!("  \x1b[90mspark ingest --compress   with tree-sitter compression\x1b[0m");
}
