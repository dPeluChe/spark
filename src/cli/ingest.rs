//! Repository ingest CLI — generate LLM-ready context files via trs.

use crate::config;
use crate::scanner::{repo_manager, repo_ingest};
use crate::scanner::repo_ingest::IngestOptions;
use crate::utils::fs::format_size;
use super::shorten_path;

#[allow(clippy::too_many_arguments)]
pub fn cmd_ingest(
    query:    Option<String>,
    all:      bool,
    compress: bool,
    read:     bool,
    budget:   Option<String>,
    changed:  bool,
    since:    Option<String>,
    deps:     bool,
    config:   &config::SparkConfig,
) {
    if read {
        if let Some(ref q) = query {
            cmd_ingest_read(q, config);
        } else {
            eprintln!("  Specify a repo: spark ingest <repo> --read");
            std::process::exit(1);
        }
        return;
    }

    if query.is_none() && !all {
        cmd_ingest_list(config);
        return;
    }

    if !repo_ingest::is_trs_available() {
        eprintln!("  trs not found. Install it:");
        eprintln!("    npm install -g @dpeluche/trs");
        eprintln!("    or: curl -fsSL https://raw.githubusercontent.com/dPeluChe/trs/main/scripts/install.sh | sh");
        std::process::exit(1);
    }

    let opts = IngestOptions { compress, budget, changed, since, deps };
    let repos = repo_manager::list_managed_repos(&config.repos_root);

    if let Some(q) = query {
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
        generate_one(found[0], &opts);

    } else if all {
        println!("  SPARK Ingest — generating LLM context for {} repos\n", repos.len());
        let (mut ok, mut skipped, mut errors) = (0u32, 0u32, 0u32);

        for (i, repo) in repos.iter().enumerate() {
            eprint!("\r  [{}/{}] {}/{}          ", i + 1, repos.len(), repo.owner, repo.name);

            if let Some(info) = repo_ingest::ingest_info(&repo.host, &repo.owner, &repo.name) {
                if info.age_secs.map(|a| a < 3600 * 24).unwrap_or(false) {
                    skipped += 1;
                    continue;
                }
            }

            match repo_ingest::generate_ingest(&repo.path, &repo.host, &repo.owner, &repo.name, &opts) {
                Ok(_)  => ok += 1,
                Err(_) => errors += 1,
            }
        }
        eprintln!("\r{}\r", " ".repeat(60));
        println!("  {} generated, {} skipped (recent), {} errors", ok, skipped, errors);
    }
}

fn generate_one(repo: &repo_manager::ManagedRepo, opts: &IngestOptions) {
    let mut flags = vec!["trs ingest"];
    if opts.compress                  { flags.push("-l aggressive"); }
    if opts.changed                   { flags.push("--changed"); }
    if opts.deps                      { flags.push("--deps"); }
    if opts.budget.is_some()          { flags.push("--budget <n>"); }
    if opts.since.is_some()           { flags.push("--since <ref>"); }

    println!("  Ingest via {}", flags.join(" "));
    println!("  \x1b[90mgithub.com/dPeluChe/trs\x1b[0m\n");

    if let Some(ref budget) = opts.budget { println!("  Budget: {}", budget); }
    if let Some(ref since)  = opts.since  { println!("  Since:  {}", since);  }

    eprint!("  Analyzing {}/{}...", repo.owner, repo.name);

    match repo_ingest::generate_ingest(&repo.path, &repo.host, &repo.owner, &repo.name, opts) {
        Ok(path) => {
            eprintln!(" done\n");
            let size  = repo_ingest::ingest_info(&repo.host, &repo.owner, &repo.name)
                .map(|i| format_size(i.size)).unwrap_or_default();
            let short = shorten_path(&path.display().to_string());
            println!("  Output: {} ({})", short, size);
            println!("\n  \x1b[90mspark ingest {} --read            print to stdout\x1b[0m", repo.name);
            println!("  \x1b[90mspark ingest {} --budget 32k       fit to context window\x1b[0m", repo.name);
            println!("  \x1b[90mspark ingest {} --changed          only uncommitted files\x1b[0m", repo.name);
            println!("  \x1b[90mspark ingest {} --deps             dependency graph only\x1b[0m", repo.name);
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
    let q     = query.to_lowercase();

    let repo = repos.iter().find(|r| {
        r.name.to_lowercase() == q
            || format!("{}/{}", r.owner, r.name).to_lowercase() == q
            || r.name.to_lowercase().contains(&q)
    });

    let repo = match repo {
        Some(r) => r,
        None => { eprintln!("  No repo matching '{}'", query); std::process::exit(1); }
    };

    let path = repo_ingest::ingest_path(&repo.host, &repo.owner, &repo.name);
    if !path.exists() {
        eprintln!("  No digest for {}/{}. Run: spark ingest {}", repo.owner, repo.name, repo.name);
        std::process::exit(1);
    }

    match std::fs::read_to_string(&path) {
        Ok(content) => {
            for line in content.lines() {
                if !is_binary_line(line) { println!("{}", line); }
            }
        }
        Err(e) => { eprintln!("  Error reading digest: {}", e); std::process::exit(1); }
    }
}

fn is_binary_line(line: &str) -> bool {
    let t = line.trim();
    if t.contains("data:image/") || t.contains("data:application/") { return true; }
    t.len() > 200 && t.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
}

fn cmd_ingest_list(config: &config::SparkConfig) {
    let repos   = repo_manager::list_managed_repos(&config.repos_root);
    let ingests = repo_ingest::list_ingests();
    let base    = dirs::config_dir().unwrap_or_default().join("spark").join("ingest");

    let common_host  = ingests.first().map(|(h, _, _, _)| h.as_str()).unwrap_or("github.com");
    let all_same_host = ingests.iter().all(|(h, _, _, _)| h == common_host);

    let trs_status = if repo_ingest::is_trs_available() {
        "\x1b[32mtrs\x1b[0m"
    } else {
        "\x1b[31mtrs not installed\x1b[0m"
    };
    println!("  \x1b[1mLLM Ingest\x1b[0m — {}/{} repos  \x1b[90m({})\x1b[0m",
        ingests.len(), repos.len(), trs_status);

    if all_same_host && !ingests.is_empty() {
        println!("  \x1b[90m{}/{}\x1b[0m\n", shorten_path(&base.display().to_string()), common_host);
    } else {
        println!("  \x1b[90m{}\x1b[0m\n", shorten_path(&base.display().to_string()));
    }

    if ingests.is_empty() {
        println!("  No digests yet.");
        println!("  spark ingest <repo>       generate for a repo");
        println!("  spark ingest --all        generate for all repos");
        if !repo_ingest::is_trs_available() {
            println!("\n  Install trs first:  npm install -g @dpeluche/trs");
        }
        return;
    }

    let max_name = ingests.iter()
        .map(|(_, o, n, _)| o.len() + 1 + n.len() + 3)
        .max().unwrap_or(20) + 2;

    let cache = repo_manager::load_status_cache();

    for (host, owner, name, info) in &ingests {
        let size = format_size(info.size);
        let age  = info.age_display();

        let status = repos.iter()
            .find(|r| r.host == *host && r.owner == *owner && r.name == *name)
            .map(|r| {
                let behind = matches!(
                    cache.get(&r.path.display().to_string()),
                    Some((s, ts)) if repo_manager::is_cache_valid(*ts)
                        && (s.starts_with("behind") || s.starts_with("diverged"))
                );
                if behind { "\x1b[33mstale\x1b[0m" } else { "\x1b[32mfresh\x1b[0m" }
            })
            .unwrap_or("\x1b[90m?\x1b[0m");

        let display_name = if all_same_host {
            format!("{}/{}.md", owner, name)
        } else {
            format!("{}/{}/{}.md", host, owner, name)
        };
        let padding = " ".repeat(max_name.saturating_sub(display_name.len()).max(2));
        println!("  {}\x1b[90m.md\x1b[0m{}{}  {:>8}  {}",
            &display_name[..display_name.len()-3], padding, status, size, age);
    }

    let missing: Vec<_> = repos.iter()
        .filter(|r| !ingests.iter().any(|(h, o, n, _)| *h == r.host && *o == r.owner && *n == r.name))
        .collect();

    if !missing.is_empty() && missing.len() <= 10 {
        println!("\n  \x1b[90mNo digest ({}):\x1b[0m", missing.len());
        for r in &missing { println!("    \x1b[90m{}/{}\x1b[0m", r.owner, r.name); }
    } else if !missing.is_empty() {
        println!("\n  \x1b[90m{} repos without digest — spark ingest --all\x1b[0m", missing.len());
    }

    println!("\n  \x1b[90mspark ingest <repo>                 regenerate\x1b[0m");
    println!("  \x1b[90mspark ingest <repo> --budget 32k    fit to context window\x1b[0m");
    println!("  \x1b[90mspark ingest <repo> --changed       only uncommitted files\x1b[0m");
    println!("  \x1b[90mspark ingest <repo> --deps          dependency graph only\x1b[0m");
}
