//! Repository management CLI commands — clone, list, search, cd, rm, status, pull.
//!
//! Submodules:
//! - status.rs — spark status (cached + fresh check)
//! - pull.rs   — spark pull (single / all / tag)

mod pull;
mod status;

pub use pull::cmd_pull;
pub use status::cmd_status;

use super::{expand_url, filter_repo, shorten_path};
use crate::config;
use crate::scanner;
use std::collections::BTreeMap;
use std::io;
use std::path::PathBuf;

pub fn cmd_clone(
    url: &str,
    ssh: bool,
    shallow: bool,
    config: &config::SparkConfig,
) -> color_eyre::Result<()> {
    let full_url = expand_url(url, ssh);
    println!("  Cloning into {}/", config.repos_root.display());

    let clone_result = if shallow {
        scanner::repo_manager::clone_repo_shallow(&full_url, &config.repos_root)
    } else {
        scanner::repo_manager::clone_repo(&full_url, &config.repos_root)
    };

    match clone_result {
        Ok(path) => {
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let display = shorten_path(&path.display().to_string());
            println!("  + {}\n", display);
            println!("  cd {}", path.display());
            println!("  alias {}='cd {}'", name.replace('-', "_"), display);
        }
        Err(e) => {
            eprintln!("  x {}", e);
            std::process::exit(1);
        }
    }
    Ok(())
}

pub fn cmd_list(
    full_path: bool,
    query: Option<String>,
    config: &config::SparkConfig,
) -> color_eyre::Result<()> {
    let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
    if repos.is_empty() {
        println!("  No repos in {}", config.repos_root.display());
        println!("  Use: spark clone <url>");
        return Ok(());
    }

    let filtered: Vec<_> = match &query {
        Some(q) => {
            let q = q.to_lowercase();
            repos.iter().filter(|r| filter_repo(r, &q)).collect()
        }
        None => repos.iter().collect(),
    };

    if full_path {
        for repo in &filtered {
            println!("{}", repo.path.display());
        }
    } else {
        print_repos_tree(&filtered);
        let tags = scanner::repo_tags::load_tags();
        let tag_count = tags.all_tags().len();
        if tag_count > 0 {
            println!("\n  \x1b[90m{} tags — spark status --tag <name> | spark pull all --tag <name>\x1b[0m", tag_count);
        }
        println!("  \x1b[90mspark tag add <repo> <tag>    add tag to a repo\x1b[0m");
        println!("  \x1b[90mspark tag list               see all tags\x1b[0m");
    }
    Ok(())
}

pub fn cmd_rm(query: &str, config: &config::SparkConfig) -> color_eyre::Result<()> {
    let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
    let q = query.to_lowercase();
    let matches: Vec<_> = repos
        .iter()
        .filter(|r| {
            r.name.to_lowercase() == q
                || format!("{}/{}", r.owner, r.name).to_lowercase() == q
                || format!("{}/{}/{}", r.host, r.owner, r.name).to_lowercase() == q
                || r.path.display().to_string().to_lowercase().contains(&q)
        })
        .collect();

    if matches.is_empty() {
        eprintln!("  No repo matching '{}' found", query);
        std::process::exit(1);
    }
    if matches.len() > 1 {
        eprintln!("  Ambiguous: {} repos match '{}'", matches.len(), query);
        for m in &matches {
            eprintln!("    {}/{}/{}", m.host, m.owner, m.name);
        }
        std::process::exit(1);
    }

    let repo = matches[0];
    println!("  Remove {}/{}/{}?", repo.host, repo.owner, repo.name);
    println!("  Path: {}", repo.path.display());
    print!("  Confirm (y/N): ");
    io::Write::flush(&mut io::stdout())?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    if answer.trim().to_lowercase() == "y" {
        if config.use_trash {
            let trash_dir = dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("spark-trash");
            std::fs::create_dir_all(&trash_dir)?;
            let dest = trash_dir.join(repo.name.clone());
            std::fs::rename(&repo.path, &dest)?;
            println!("  Moved to trash: {}", dest.display());
        } else {
            std::fs::remove_dir_all(&repo.path)?;
            println!("  Removed: {}", repo.path.display());
        }
    } else {
        println!("  Cancelled");
    }
    Ok(())
}

pub fn cmd_search(
    query: &str,
    first: bool,
    config: &config::SparkConfig,
) -> color_eyre::Result<()> {
    let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
    let q = query.to_lowercase();
    let matches: Vec<_> = repos.iter().filter(|r| filter_repo(r, &q)).collect();

    if matches.is_empty() {
        eprintln!("  No repos matching '{}'", query);
        std::process::exit(1);
    }

    if first {
        println!("{}", matches[0].path.display());
    } else {
        let cache = scanner::repo_manager::load_status_cache();
        for (i, repo) in matches.iter().enumerate() {
            if i > 0 {
                println!();
            }
            print_search_result(repo, &cache);
        }
    }
    Ok(())
}

fn print_search_result(
    repo: &scanner::repo_manager::ManagedRepo,
    cache: &std::collections::HashMap<String, (String, u64)>,
) {
    let short = shorten_path(&repo.path.display().to_string());
    let age = repo.last_commit.as_deref().unwrap_or("-");
    let key = repo.path.display().to_string();
    let status = cache
        .get(&key)
        .filter(|(_, ts)| scanner::repo_manager::is_cache_valid(*ts))
        .map(|(s, _)| scanner::repo_manager::string_to_status(s));
    let status_str = status
        .as_ref()
        .map(|s| format!("{}", s))
        .unwrap_or_else(|| "unknown (run spark status)".into());

    println!("  repo:   {}", repo.name);
    println!("  owner:  {}", repo.owner);
    println!("  host:   {}", repo.host);
    println!("  branch: {}", repo.branch);
    println!("  status: {}", status_str);
    println!("  commit: {}", age);
    println!("  path:   {}", short);
    if let Some(info) = scanner::repo_ingest::ingest_info(&repo.owner, &repo.name) {
        println!(
            "  ingest: {} ({}, {})",
            shorten_path(&info.path.display().to_string()),
            crate::utils::fs::format_size(info.size),
            info.age_display()
        );
    }
}

pub fn cmd_cd(query: &str, config: &config::SparkConfig) -> color_eyre::Result<()> {
    let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
    let q = query.to_lowercase();
    let exact = repos.iter().find(|r| r.name.to_lowercase() == q);
    let found = exact.or_else(|| repos.iter().find(|r| r.name.to_lowercase().contains(&q)));

    match found {
        Some(repo) => println!("{}", repo.path.display()),
        None => {
            eprintln!("  No repo matching '{}'", query);
            std::process::exit(1);
        }
    }
    Ok(())
}

fn print_repos_tree(repos: &[&scanner::repo_manager::ManagedRepo]) {
    let tags = scanner::repo_tags::load_tags();

    struct RepoEntry<'a> {
        name: &'a str,
        branch: &'a str,
        age: &'a str,
        tags: Vec<String>,
    }

    let mut tree: BTreeMap<&str, BTreeMap<&str, Vec<RepoEntry>>> = BTreeMap::new();
    for r in repos {
        let age = r.last_commit.as_deref().unwrap_or("-");
        let key = scanner::repo_tags::repo_key(&r.host, &r.owner, &r.name);
        let repo_tags = tags.tags_for_repo(&key);
        tree.entry(&r.host)
            .or_default()
            .entry(&r.owner)
            .or_default()
            .push(RepoEntry {
                name: &r.name,
                branch: &r.branch,
                age,
                tags: repo_tags,
            });
    }
    for owners in tree.values_mut() {
        for entries in owners.values_mut() {
            entries.sort_by_key(|e| e.name.to_lowercase());
        }
    }

    // Fixed info column at 50, extended if the longest name + prefix wouldn't fit
    let longest = tree
        .values()
        .flat_map(|owners| {
            owners
                .values()
                .flat_map(|entries| entries.iter().map(|e| 8 + e.name.len()))
        })
        .max()
        .unwrap_or(30);
    let info_col = longest.max(50) + 2;

    for (host, owners) in &tree {
        println!("{}", host);
        let oc = owners.len();
        for (oi, (owner, entries)) in owners.iter().enumerate() {
            let lo = oi == oc - 1;
            println!("{}{}", if lo { "└── " } else { "├── " }, owner);
            let pf = if lo { "    " } else { "│   " };
            for (ni, e) in entries.iter().enumerate() {
                let connector = if ni == entries.len() - 1 {
                    "└── "
                } else {
                    "├── "
                };
                let name_part = format!("{}{}{}", pf, connector, e.name);
                let padding = if name_part.len() < info_col {
                    " ".repeat(info_col - name_part.len())
                } else {
                    "  ".to_string()
                };
                let tag_str = if e.tags.is_empty() {
                    String::new()
                } else {
                    format!("  \x1b[36m[{}]\x1b[0m", e.tags.join(","))
                };
                println!(
                    "{}{}\x1b[35m{}\x1b[0m  \x1b[90m{}\x1b[0m{}",
                    name_part, padding, e.branch, e.age, tag_str
                );
            }
        }
    }
}
