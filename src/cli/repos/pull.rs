//! `spark pull` — pull repos by name, `all`, or `--tag`.

use super::super::filter_repo;
use crate::config;
use crate::scanner;

pub fn cmd_pull(query: &str, tag: Option<String>, config: &config::SparkConfig) {
    let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);

    let filtered: Vec<_> = if let Some(ref tag_name) = tag {
        let tags = scanner::repo_tags::load_tags();
        let tag_repos = tags.repos_for_tag(tag_name);
        if tag_repos.is_empty() {
            eprintln!("  No repos with tag '{}'", tag_name);
            std::process::exit(1);
        }
        println!("  Tag: {}", tag_name);
        repos
            .iter()
            .filter(|r| {
                let key = scanner::repo_tags::repo_key(&r.host, &r.owner, &r.name);
                tag_repos.contains(&key)
            })
            .collect()
    } else if query.to_lowercase() == "all" {
        repos.iter().collect()
    } else {
        let q = query.to_lowercase();
        let exact: Vec<_> = repos
            .iter()
            .filter(|r| {
                format!("{}/{}", r.owner, r.name).to_lowercase() == q || r.name.to_lowercase() == q
            })
            .collect();
        if !exact.is_empty() {
            exact
        } else {
            repos.iter().filter(|r| filter_repo(r, &q)).collect()
        }
    };
    let is_all = tag.is_some() || query.to_lowercase() == "all";

    if filtered.is_empty() {
        eprintln!("  No repos matching '{}'", query);
        std::process::exit(1);
    }
    if !is_all && filtered.len() > 1 {
        eprintln!(
            "  {} repos match '{}'. Be more specific:\n",
            filtered.len(),
            query
        );
        for r in &filtered {
            eprintln!("    spark pull {}/{}", r.owner, r.name);
        }
        std::process::exit(1);
    }

    println!("  Checking {} repos for updates...\n", filtered.len());
    let (pulled, skipped, errors) = pull_all(&filtered);
    eprintln!("\r{}\r", " ".repeat(60));

    if pulled > 0 {
        println!("  {} repos pulled", pulled);
    }
    if skipped > 0 {
        println!("  {} repos already up to date", skipped);
    }
    for e in &errors {
        eprintln!("  Error: {}", e);
    }
    if pulled == 0 && errors.is_empty() {
        println!("  All repos up to date");
    }
}

fn pull_all(filtered: &[&scanner::repo_manager::ManagedRepo]) -> (usize, usize, Vec<String>) {
    let mut pulled = 0usize;
    let mut skipped = 0usize;
    let mut errors = Vec::new();

    for (i, repo) in filtered.iter().enumerate() {
        eprint!(
            "\r  [{}/{}] {}/{}",
            i + 1,
            filtered.len(),
            repo.owner,
            repo.name
        );
        let status = scanner::repo_manager::check_repo_status(&repo.path);
        let key = repo.path.display().to_string();
        match &status {
            scanner::repo_manager::RepoStatus::Behind(_)
            | scanner::repo_manager::RepoStatus::Diverged { .. } => {
                match scanner::repo_manager::pull_repo(&repo.path) {
                    Ok(_) => {
                        pulled += 1;
                        scanner::repo_manager::save_status_to_cache(&key, "up_to_date");
                    }
                    Err(e) => {
                        errors.push(format!("{}/{}: {}", repo.owner, repo.name, e));
                    }
                }
            }
            _ => {
                skipped += 1;
                scanner::repo_manager::save_status_to_cache(
                    &key,
                    &scanner::repo_manager::status_to_string(&status),
                );
            }
        }
    }
    (pulled, skipped, errors)
}
