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
        let tags = scanner::repo_tags::load_tags();
        let tag_count = tags.all_tags().len();
        if tag_count > 0 {
            println!("\n  \x1b[90m{} tags defined — spark status --tag <name> | spark pull all --tag <name>\x1b[0m", tag_count);
        } else {
            println!("\n  \x1b[90mTip: spark tag add <repo> <tag> to organize repos into groups\x1b[0m");
        }
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

    // Sort: up-to-date first (alphabetic), then outdated (alphabetic)
    statuses.sort_by(|a, b| {
        let a_ok = matches!(a.1, scanner::repo_manager::RepoStatus::UpToDate);
        let b_ok = matches!(b.1, scanner::repo_manager::RepoStatus::UpToDate);
        b_ok.cmp(&a_ok)
            .then(a.0.owner.to_lowercase().cmp(&b.0.owner.to_lowercase()))
            .then(a.0.name.to_lowercase().cmp(&b.0.name.to_lowercase()))
    });

    print_status_table(&statuses);

    let needs = statuses.iter().filter(|(_, s)| !matches!(s, scanner::repo_manager::RepoStatus::UpToDate)).count();
    let updated = statuses.iter().filter(|(_, s)| matches!(s, scanner::repo_manager::RepoStatus::UpToDate)).count();
    println!("\n  {} total — {} need pull, {} up to date", statuses.len(), needs, updated);
    if needs > 0 {
        println!("  spark pull <name>          pull a specific repo");
        println!("  spark pull all             pull all behind repos");
        println!("  spark pull all --tag <t>   pull repos by tag");
    }
    let all_tags = scanner::repo_tags::load_tags().all_tags();
    if !all_tags.is_empty() {
        println!("\n  \x1b[90mTags: {}\x1b[0m", all_tags.join(", "));
    } else {
        println!("\n  \x1b[90mTip: spark tag add <repo> <tag> to organize repos into groups\x1b[0m");
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
    let tags = scanner::repo_tags::load_tags();

    struct RepoEntry<'a> { name: &'a str, branch: &'a str, age: &'a str, tags: Vec<String> }

    let mut tree: BTreeMap<&str, BTreeMap<&str, Vec<RepoEntry>>> = BTreeMap::new();
    for r in repos {
        let age = r.last_commit.as_deref().unwrap_or("-");
        let key = scanner::repo_tags::repo_key(&r.host, &r.owner, &r.name);
        let repo_tags = tags.tags_for_repo(&key);
        tree.entry(&r.host).or_default().entry(&r.owner).or_default()
            .push(RepoEntry { name: &r.name, branch: &r.branch, age, tags: repo_tags });
    }
    for owners in tree.values_mut() {
        for entries in owners.values_mut() { entries.sort_by_key(|e| e.name.to_lowercase()); }
    }

    // Fixed column at 50, but ensure longest name fits with at least 2 spaces
    let longest = tree.values().flat_map(|owners| {
        owners.iter().flat_map(|(_, entries)| {
            entries.iter().map(|e| 8 + e.name.len())
        })
    }).max().unwrap_or(30);
    let info_col = longest.max(50) + 2;

    for (host, owners) in &tree {
        println!("{}", host);
        let oc = owners.len();
        for (oi, (owner, entries)) in owners.iter().enumerate() {
            let lo = oi == oc - 1;
            println!("{}{}", if lo { "└── " } else { "├── " }, owner);
            let pf = if lo { "    " } else { "│   " };
            for (ni, e) in entries.iter().enumerate() {
                let connector = if ni == entries.len() - 1 { "└── " } else { "├── " };
                let name_part = format!("{}{}{}", pf, connector, e.name);
                let padding = if name_part.len() < info_col {
                    " ".repeat(info_col - name_part.len())
                } else {
                    "  ".to_string()
                };
                let tag_str = if e.tags.is_empty() { String::new() }
                    else { format!("  \x1b[36m[{}]\x1b[0m", e.tags.join(",")) };
                println!("{}{}\x1b[35m{}\x1b[0m  \x1b[90m{}\x1b[0m{}",
                    name_part, padding, e.branch, e.age, tag_str);
            }
        }
    }
}

fn print_status_table(statuses: &[(&scanner::repo_manager::ManagedRepo, scanner::repo_manager::RepoStatus)]) {
    let tags = scanner::repo_tags::load_tags();
    let max_name = statuses.iter()
        .map(|(r, _)| r.owner.len() + 1 + r.name.len())
        .max().unwrap_or(20) + 2;

    let needs_attention: Vec<_> = statuses.iter()
        .filter(|(_, s)| !matches!(s, scanner::repo_manager::RepoStatus::UpToDate))
        .collect();
    let up_to_date: Vec<_> = statuses.iter()
        .filter(|(_, s)| matches!(s, scanner::repo_manager::RepoStatus::UpToDate))
        .collect();

    if !needs_attention.is_empty() {
        println!("  \x1b[33mNeeds attention ({})\x1b[0m\n", needs_attention.len());
        for (repo, status) in &needs_attention {
            let key = scanner::repo_tags::repo_key(&repo.host, &repo.owner, &repo.name);
            let repo_tags_list = tags.tags_for_repo(&key);
            print_status_row(repo, status, max_name, &repo_tags_list);
        }
        println!();
    }

    if !up_to_date.is_empty() {
        println!("  \x1b[32mUp to date ({})\x1b[0m\n", up_to_date.len());
        for (repo, status) in &up_to_date {
            let key = scanner::repo_tags::repo_key(&repo.host, &repo.owner, &repo.name);
            let repo_tags_list = tags.tags_for_repo(&key);
            print_status_row(repo, status, max_name, &repo_tags_list);
        }
    }
}

fn print_status_row(repo: &scanner::repo_manager::ManagedRepo, status: &scanner::repo_manager::RepoStatus, max_name: usize, repo_tags: &[String]) {
    let indicator = match status {
        scanner::repo_manager::RepoStatus::UpToDate => "+",
        scanner::repo_manager::RepoStatus::Behind(_) => "v",
        scanner::repo_manager::RepoStatus::Ahead(_) => "^",
        scanner::repo_manager::RepoStatus::Diverged { .. } => "~",
        scanner::repo_manager::RepoStatus::Dirty => "*",
        scanner::repo_manager::RepoStatus::Error(_) => "x",
        scanner::repo_manager::RepoStatus::Checking => "?",
    };
    let repo_name = format!("{}/{}", repo.owner, repo.name);
    let age = repo.last_commit.as_deref().unwrap_or("-");
    let status_str = format!("{}", status);
    let tag_str = if repo_tags.is_empty() { String::new() }
        else { format!("  \x1b[36m[{}]\x1b[0m", repo_tags.join(",")) };

    println!("  {:<width$}   {}   {:<14}  {}{}",
        repo_name, indicator, status_str, age, tag_str, width = max_name);
}
