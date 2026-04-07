//! Repository management CLI commands: clone, list, search, cd, rm, status, pull.

use std::io;
use std::path::PathBuf;
use std::collections::BTreeMap;
use crate::config;
use crate::scanner;
use super::{filter_repo, shorten_path, expand_url};

pub fn cmd_clone(url: &str, ssh: bool, shallow: bool, config: &config::SparkConfig) -> color_eyre::Result<()> {
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

pub fn cmd_list(full_path: bool, query: Option<String>, config: &config::SparkConfig) -> color_eyre::Result<()> {
    let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
    if repos.is_empty() {
        println!("  No repos in {}", config.repos_root.display());
        println!("  Use: spark clone <url>");
        return Ok(());
    }

    let filtered: Vec<_> = match &query {
        Some(q) => { let q = q.to_lowercase(); repos.iter().filter(|r| filter_repo(r, &q)).collect() }
        None => repos.iter().collect(),
    };

    if full_path {
        for repo in &filtered { println!("{}", repo.path.display()); }
    } else {
        print_repos_tree(&filtered);
    }
    Ok(())
}

pub fn cmd_rm(query: &str, config: &config::SparkConfig) -> color_eyre::Result<()> {
    let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
    let q = query.to_lowercase();
    let matches: Vec<_> = repos.iter().filter(|r| {
        r.name.to_lowercase() == q
            || format!("{}/{}", r.owner, r.name).to_lowercase() == q
            || format!("{}/{}/{}", r.host, r.owner, r.name).to_lowercase() == q
            || r.path.display().to_string().to_lowercase().contains(&q)
    }).collect();

    if matches.is_empty() { eprintln!("  No repo matching '{}' found", query); std::process::exit(1); }
    if matches.len() > 1 {
        eprintln!("  Ambiguous: {} repos match '{}'", matches.len(), query);
        for m in &matches { eprintln!("    {}/{}/{}", m.host, m.owner, m.name); }
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
            let trash_dir = dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join("spark-trash");
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

pub fn cmd_search(query: &str, first: bool, config: &config::SparkConfig) -> color_eyre::Result<()> {
    let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
    let q = query.to_lowercase();
    let matches: Vec<_> = repos.iter().filter(|r| filter_repo(r, &q)).collect();

    if matches.is_empty() { eprintln!("  No repos matching '{}'", query); std::process::exit(1); }

    if first {
        println!("{}", matches[0].path.display());
    } else {
        let cache = scanner::repo_manager::load_status_cache();
        for (i, repo) in matches.iter().enumerate() {
            if i > 0 { println!(); }
            let short = shorten_path(&repo.path.display().to_string());
            let age = repo.last_commit.as_deref().unwrap_or("-");
            let key = repo.path.display().to_string();
            let status = cache.get(&key)
                .filter(|(_, ts)| scanner::repo_manager::is_cache_valid(*ts))
                .map(|(s, _)| scanner::repo_manager::string_to_status(s));
            let status_str = status.as_ref()
                .map(|s| format!("{}", s))
                .unwrap_or_else(|| "unknown (run spark status)".into());

            println!("  repo:   {}", repo.name);
            println!("  owner:  {}", repo.owner);
            println!("  host:   {}", repo.host);
            println!("  branch: {}", repo.branch);
            println!("  status: {}", status_str);
            println!("  commit: {}", age);
            println!("  path:   {}", short);
        }
    }
    Ok(())
}

pub fn cmd_cd(query: &str, config: &config::SparkConfig) -> color_eyre::Result<()> {
    let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
    let q = query.to_lowercase();
    let exact = repos.iter().find(|r| r.name.to_lowercase() == q);
    let found = exact.or_else(|| repos.iter().find(|r| r.name.to_lowercase().contains(&q)));

    match found {
        Some(repo) => println!("{}", repo.path.display()),
        None => { eprintln!("  No repo matching '{}'", query); std::process::exit(1); }
    }
    Ok(())
}

pub fn cmd_status(query: Option<String>, tag: Option<String>, config: &config::SparkConfig) {
    let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
    let tag_filter = tag.as_ref().map(|t| {
        let tags = scanner::repo_tags::load_tags();
        tags.repos_for_tag(t)
    });

    let filtered: Vec<_> = if let Some(ref tag_repos) = tag_filter {
        repos.iter().filter(|r| {
            let key = scanner::repo_tags::repo_key(&r.host, &r.owner, &r.name);
            tag_repos.contains(&key)
        }).collect()
    } else {
        match &query {
            Some(q) => { let q = q.to_lowercase(); repos.iter().filter(|r| filter_repo(r, &q)).collect() }
            None => repos.iter().collect(),
        }
    };

    if let Some(t) = &tag {
        if filtered.is_empty() { println!("  No repos with tag '{}'", t); return; }
        println!("  Tag: {}", t);
    }

    if filtered.is_empty() { println!("  No repos found"); return; }
    println!("  Checking {} repos...\n", filtered.len());

    let cache = scanner::repo_manager::load_status_cache();
    let mut statuses: Vec<(&scanner::repo_manager::ManagedRepo, scanner::repo_manager::RepoStatus)> = Vec::new();

    for (i, repo) in filtered.iter().enumerate() {
        eprint!("\r  [{}/{}] {}/{}", i + 1, filtered.len(), repo.owner, repo.name);
        let key = repo.path.display().to_string();
        let status = if let Some((cached, ts)) = cache.get(&key) {
            if scanner::repo_manager::is_cache_valid(*ts) {
                scanner::repo_manager::string_to_status(cached)
            } else {
                let s = scanner::repo_manager::check_repo_status(&repo.path);
                scanner::repo_manager::save_status_to_cache(&key, &scanner::repo_manager::status_to_string(&s));
                s
            }
        } else {
            let s = scanner::repo_manager::check_repo_status(&repo.path);
            scanner::repo_manager::save_status_to_cache(&key, &scanner::repo_manager::status_to_string(&s));
            s
        };
        statuses.push((repo, status));
    }
    eprintln!("\r{}\r", " ".repeat(60));

    print_status_table(&statuses);

    let behind = statuses.iter().filter(|(_, s)| matches!(s, scanner::repo_manager::RepoStatus::Behind(_))).count();
    let diverged = statuses.iter().filter(|(_, s)| matches!(s, scanner::repo_manager::RepoStatus::Diverged { .. })).count();
    if behind > 0 || diverged > 0 {
        println!("\n  {} repos need pull", behind + diverged);
        println!("  spark pull <name>   pull a specific repo");
        println!("  spark pull all      pull all behind repos");
    }
}

pub fn cmd_pull(query: &str, tag: Option<String>, config: &config::SparkConfig) {
    let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);

    // If --tag is provided, use it instead of query
    let filtered: Vec<_> = if let Some(ref tag_name) = tag {
        let tags = scanner::repo_tags::load_tags();
        let tag_repos = tags.repos_for_tag(tag_name);
        if tag_repos.is_empty() {
            eprintln!("  No repos with tag '{}'", tag_name);
            std::process::exit(1);
        }
        println!("  Tag: {}", tag_name);
        repos.iter().filter(|r| {
            let key = scanner::repo_tags::repo_key(&r.host, &r.owner, &r.name);
            tag_repos.contains(&key)
        }).collect()
    } else {
        let is_all = query.to_lowercase() == "all";
        if is_all {
            repos.iter().collect()
        } else {
            let q = query.to_lowercase();
            let exact: Vec<_> = repos.iter().filter(|r| {
                format!("{}/{}", r.owner, r.name).to_lowercase() == q || r.name.to_lowercase() == q
            }).collect();
            if !exact.is_empty() { exact } else { repos.iter().filter(|r| filter_repo(r, &q)).collect() }
        }
    };
    let is_all = tag.is_some() || query.to_lowercase() == "all";

    if filtered.is_empty() { eprintln!("  No repos matching '{}'", query); std::process::exit(1); }
    if !is_all && filtered.len() > 1 {
        eprintln!("  {} repos match '{}'. Be more specific:\n", filtered.len(), query);
        for r in &filtered { eprintln!("    spark pull {}/{}", r.owner, r.name); }
        std::process::exit(1);
    }

    println!("  Checking {} repos for updates...\n", filtered.len());
    let mut pulled = 0usize;
    let mut skipped = 0usize;
    let mut errors = Vec::new();

    for (i, repo) in filtered.iter().enumerate() {
        eprint!("\r  [{}/{}] {}/{}", i + 1, filtered.len(), repo.owner, repo.name);
        let status = scanner::repo_manager::check_repo_status(&repo.path);
        let key = repo.path.display().to_string();
        match &status {
            scanner::repo_manager::RepoStatus::Behind(_)
            | scanner::repo_manager::RepoStatus::Diverged { .. } => {
                match scanner::repo_manager::pull_repo(&repo.path) {
                    Ok(_) => { pulled += 1; scanner::repo_manager::save_status_to_cache(&key, "up_to_date"); }
                    Err(e) => { errors.push(format!("{}/{}: {}", repo.owner, repo.name, e)); }
                }
            }
            _ => { skipped += 1; scanner::repo_manager::save_status_to_cache(&key, &scanner::repo_manager::status_to_string(&status)); }
        }
    }
    eprintln!("\r{}\r", " ".repeat(60));

    if pulled > 0 { println!("  {} repos pulled", pulled); }
    if skipped > 0 { println!("  {} repos already up to date", skipped); }
    for e in &errors { eprintln!("  Error: {}", e); }
    if pulled == 0 && errors.is_empty() { println!("  All repos up to date"); }
}

// ─── Display helpers ───

fn print_repos_tree(repos: &[&scanner::repo_manager::ManagedRepo]) {
    let mut tree: BTreeMap<&str, BTreeMap<&str, Vec<&str>>> = BTreeMap::new();
    for r in repos {
        tree.entry(&r.host).or_default().entry(&r.owner).or_default().push(&r.name);
    }
    for owners in tree.values_mut() {
        for names in owners.values_mut() { names.sort_unstable_by_key(|a| a.to_lowercase()); }
    }
    for (host, owners) in &tree {
        println!("{}", host);
        let oc = owners.len();
        for (oi, (owner, names)) in owners.iter().enumerate() {
            let lo = oi == oc - 1;
            println!("{}{}", if lo { "└── " } else { "├── " }, owner);
            let pf = if lo { "    " } else { "│   " };
            for (ni, name) in names.iter().enumerate() {
                println!("{}{}{}", pf, if ni == names.len() - 1 { "└── " } else { "├── " }, name);
            }
        }
    }
}

fn print_status_table(statuses: &[(&scanner::repo_manager::ManagedRepo, scanner::repo_manager::RepoStatus)]) {
    struct Entry<'a> { owner: &'a str, name: &'a str, status: &'a scanner::repo_manager::RepoStatus, last_commit: &'a Option<String> }
    let mut by_host: BTreeMap<&str, Vec<Entry>> = BTreeMap::new();
    for (r, s) in statuses {
        by_host.entry(r.host.as_str()).or_default().push(Entry { owner: &r.owner, name: &r.name, status: s, last_commit: &r.last_commit });
    }
    for (host, mut entries) in by_host {
        entries.sort_by(|a, b| a.owner.to_lowercase().cmp(&b.owner.to_lowercase()).then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));
        let max_name = entries.iter().map(|e| e.owner.len() + 1 + e.name.len()).max().unwrap_or(20) + 2;
        println!("  {} — {} repos\n", host, entries.len());
        for entry in &entries {
            let indicator = match entry.status {
                scanner::repo_manager::RepoStatus::UpToDate => "+",
                scanner::repo_manager::RepoStatus::Behind(_) => "v",
                scanner::repo_manager::RepoStatus::Ahead(_) => "^",
                scanner::repo_manager::RepoStatus::Diverged { .. } => "~",
                scanner::repo_manager::RepoStatus::Dirty => "*",
                scanner::repo_manager::RepoStatus::Error(_) => "x",
                scanner::repo_manager::RepoStatus::Checking => "?",
            };
            let repo_name = format!("{}/{}", entry.owner, entry.name);
            let age = entry.last_commit.as_deref().unwrap_or("-");
            let status_str = format!("{}", entry.status);
            println!("  {:<width$}   {}   {:<14}  {}", repo_name, indicator, status_str, age, width = max_name);
        }
    }
}
