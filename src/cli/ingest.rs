//! Repository ingest CLI — generate LLM-ready context files.

use crate::config;
use crate::scanner::{repo_manager, repo_ingest};
use crate::utils::fs::format_size;
use super::shorten_path;

pub fn cmd_ingest(query: Option<String>, all: bool, compress: bool, read: bool, config: &config::SparkConfig) {
    // --read: print ingest content to stdout
    if read {
        if let Some(ref q) = query {
            cmd_ingest_read(q, config);
        } else {
            eprintln!("  Specify a repo: spark ingest <repo> --read");
            std::process::exit(1);
        }
        return;
    }

    // Default (no query, no --all) = list existing
    if query.is_none() && !all {
        cmd_ingest_list(config);
        return;
    }

    // Check npx availability (repomix runs via npx)
    if !repo_ingest::is_npx_available() {
        eprintln!("  npx not found. Install Node.js to use repomix:");
        eprintln!("    brew install node");
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
    } else if all {
        // --all: ingest all repos
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
    let compress_label = if compress { " --compress" } else { "" };
    println!("  Ingest via repomix{}", compress_label);
    println!("  \x1b[90mgithub.com/yamadashy/repomix\x1b[0m\n");

    eprint!("  Analyzing {}/{}...", repo.owner, repo.name);
    match repo_ingest::generate_ingest(&repo.path, &repo.host, &repo.owner, &repo.name, compress) {
        Ok(path) => {
            eprintln!(" done\n");
            let info = repo_ingest::ingest_info(&repo.host, &repo.owner, &repo.name);
            let size = info.as_ref().map(|i| format_size(i.size)).unwrap_or_default();
            let short = shorten_path(&path.display().to_string());
            println!("  Output: {} ({})", short, size);
            println!("\n  \x1b[90mspark ingest {} --read     print to stdout\x1b[0m", repo.name);
            println!("  \x1b[90mspark ingest {} --compress  reduce ~70%% tokens\x1b[0m", repo.name);
        }
        Err(e) => {
            eprintln!(" failed\n");
            eprintln!("  Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_ingest_read(query: &str, config: &config::SparkConfig) {
    let repos = repo_manager::list_managed_repos(&config.repos_root);
    let q = query.to_lowercase();
    let found = repos.iter().find(|r| {
        r.name.to_lowercase() == q
            || format!("{}/{}", r.owner, r.name).to_lowercase() == q
            || r.name.to_lowercase().contains(&q)
    });

    let repo = match found {
        Some(r) => r,
        None => { eprintln!("  No repo matching '{}'", query); std::process::exit(1); }
    };

    let path = repo_ingest::ingest_path(&repo.host, &repo.owner, &repo.name);
    if !path.exists() {
        eprintln!("  No ingest for {}/{}. Run: spark ingest {}", repo.owner, repo.name, repo.name);
        std::process::exit(1);
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => {
            // Filter out base64/binary lines (SVG data, images, etc.)
            for line in content.lines() {
                if is_binary_line(line) { continue; }
                println!("{}", line);
            }
        }
        Err(e) => { eprintln!("  Error reading ingest: {}", e); std::process::exit(1); }
    }
}

/// Skip lines that are base64-encoded data, binary blobs, or very long data URIs
fn is_binary_line(line: &str) -> bool {
    let trimmed = line.trim();
    // Skip base64 data URIs (common in SVG/HTML)
    if trimmed.contains("data:image/") || trimmed.contains("data:application/") {
        return true;
    }
    // Skip pure base64 blocks (long lines of only base64 chars)
    if trimmed.len() > 200 {
        let base64_chars = trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=');
        if base64_chars { return true; }
    }
    false
}

fn cmd_ingest_list(config: &config::SparkConfig) {
    let repos = repo_manager::list_managed_repos(&config.repos_root);
    let ingests = repo_ingest::list_ingests();

    let total_repos = repos.len();
    let ingested = ingests.len();

    let base_path = dirs::config_dir()
        .unwrap_or_default().join("spark").join("ingest");
    // Find common host (usually all github.com)
    let common_host = ingests.first().map(|(h, _, _, _)| h.as_str()).unwrap_or("github.com");
    let all_same_host = ingests.iter().all(|(h, _, _, _)| h == common_host);

    println!("  \x1b[1mLLM Ingest\x1b[0m — {}/{} repos have context files", ingested, total_repos);
    if all_same_host && !ingests.is_empty() {
        println!("  \x1b[90m{}/{}\x1b[0m\n", shorten_path(&base_path.display().to_string()), common_host);
    } else {
        println!("  \x1b[90m{}\x1b[0m\n", shorten_path(&base_path.display().to_string()));
    }

    if ingests.is_empty() {
        println!("  No ingest files yet.");
        println!("  spark ingest <repo>       generate for a repo");
        println!("  spark ingest --all        generate for all repos");
        return;
    }

    // +3 for .md suffix in display
    let max_name = ingests.iter()
        .map(|(_, o, n, _)| o.len() + 1 + n.len() + 3)
        .max().unwrap_or(20) + 2;

    let cache = repo_manager::load_status_cache();

    for (host, owner, name, info) in &ingests {
        let size = format_size(info.size);
        let age = info.age_display();

        let repo = repos.iter().find(|r| r.host == *host && r.owner == *owner && r.name == *name);
        let status = if let Some(r) = repo {
            let repo_path_str = r.path.display().to_string();
            let is_behind = match cache.get(&repo_path_str) {
                Some((s, ts)) if repo_manager::is_cache_valid(*ts) => {
                    s.starts_with("behind") || s.starts_with("diverged")
                }
                _ => false,
            };
            if is_behind { "\x1b[33mstale\x1b[0m" } else { "\x1b[32mfresh\x1b[0m" }
        } else { "\x1b[90m?\x1b[0m" };

        let display_name = if all_same_host {
            format!("{}/{}.md", owner, name)
        } else {
            format!("{}/{}/{}.md", host, owner, name)
        };
        let padding = if display_name.len() < max_name {
            " ".repeat(max_name - display_name.len())
        } else {
            "  ".to_string()
        };
        println!("  {}\x1b[90m{}\x1b[0m{}{}  {:>8}  {}",
            &display_name[..display_name.len()-3], ".md", padding, status, size, age);
    }

    // Show repos without ingest
    let missing: Vec<_> = repos.iter().filter(|r| {
        !ingests.iter().any(|(h, o, n, _)| *h == r.host && *o == r.owner && *n == r.name)
    }).collect();

    if !missing.is_empty() && missing.len() <= 10 {
        println!("\n  \x1b[90mNo ingest ({}):\x1b[0m", missing.len());
        for r in &missing {
            println!("    \x1b[90m{}/{}\x1b[0m", r.owner, r.name);
        }
    } else if !missing.is_empty() {
        println!("\n  \x1b[90m{} repos without ingest — spark ingest --all\x1b[0m", missing.len());
    }

    println!("\n  \x1b[90mspark ingest <repo>          regenerate one\x1b[0m");
    println!("  \x1b[90mspark ingest --all           generate all\x1b[0m");
    println!("  \x1b[90mspark ingest <r> --compress   with tree-sitter (~70%% less tokens)\x1b[0m");
}
