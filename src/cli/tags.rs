//! Repository tag management CLI commands.

use crate::config;
use crate::scanner::{repo_manager, repo_tags};
use super::TagAction;

pub fn cmd_tag(action: TagAction, config: &config::SparkConfig) {
    let mut tags = repo_tags::load_tags();
    let repos = repo_manager::list_managed_repos(&config.repos_root);

    match action {
        TagAction::Add { repo, tag } => {
            let found = find_repo(&repos, &repo);
            match found {
                Some(r) => {
                    let key = repo_tags::repo_key(&r.host, &r.owner, &r.name);
                    tags.add(&key, &tag);
                    repo_tags::save_tags(&tags);
                    println!("  Tagged {}/{} with '{}'", r.owner, r.name, tag);
                }
                None => {
                    eprintln!("  No repo matching '{}'", repo);
                    suggest_repos(&repos, &repo);
                    std::process::exit(1);
                }
            }
        }

        TagAction::Remove { repo, tag } => {
            let found = find_repo(&repos, &repo);
            match found {
                Some(r) => {
                    let key = repo_tags::repo_key(&r.host, &r.owner, &r.name);
                    if tags.remove(&key, &tag) {
                        repo_tags::save_tags(&tags);
                        println!("  Removed tag '{}' from {}/{}", tag, r.owner, r.name);
                    } else {
                        eprintln!("  {}/{} doesn't have tag '{}'", r.owner, r.name, tag);
                    }
                }
                None => {
                    eprintln!("  No repo matching '{}'", repo);
                    std::process::exit(1);
                }
            }
        }

        TagAction::List { tag } => {
            if let Some(tag_name) = tag {
                // Show repos in a specific tag
                let repo_keys = tags.repos_for_tag(&tag_name);
                if repo_keys.is_empty() {
                    println!("  No repos tagged '{}'", tag_name);
                    return;
                }
                println!("  \x1b[1m{}\x1b[0m ({} repos)\n", tag_name, repo_keys.len());
                for key in &repo_keys {
                    // Find matching repo for extra info
                    let info = repos.iter().find(|r| {
                        repo_tags::repo_key(&r.host, &r.owner, &r.name) == *key
                    });
                    if let Some(r) = info {
                        let age = r.last_commit.as_deref().unwrap_or("-");
                        println!("    {}/{}  \x1b[90m{}\x1b[0m", r.owner, r.name, age);
                    } else {
                        println!("    \x1b[90m{} (not found in repos root)\x1b[0m", key);
                    }
                }
            } else {
                // Show all tags with counts
                let all = tags.all_tags();
                if all.is_empty() {
                    println!("  No tags defined yet.");
                    println!("  Use: spark tag add <repo> <tag>");
                    return;
                }
                println!("  \x1b[1m{} tags\x1b[0m\n", all.len());
                for tag_name in &all {
                    let count = tags.repos_for_tag(tag_name).len();
                    println!("    \x1b[33m{}\x1b[0m  {} repos", tag_name, count);
                }
                println!("\n  Use: spark tag list <name> to see repos in a tag");
            }
        }

        TagAction::Delete { tag } => {
            let count = tags.repos_for_tag(&tag).len();
            if count == 0 {
                eprintln!("  Tag '{}' not found", tag);
                std::process::exit(1);
            }
            let new_tags = tags.delete_tag(&tag);
            repo_tags::save_tags(&new_tags);
            println!("  Deleted tag '{}' ({} repos untagged)", tag, count);
        }

        TagAction::Rename { old, new_name } => {
            if tags.repos_for_tag(&old).is_empty() {
                eprintln!("  Tag '{}' not found", old);
                std::process::exit(1);
            }
            tags.rename_tag(&old, &new_name);
            repo_tags::save_tags(&tags);
            println!("  Renamed tag '{}' -> '{}'", old, new_name);
        }
    }
}

/// Find a repo by name, owner/name, or host/owner/name
fn find_repo<'a>(repos: &'a [repo_manager::ManagedRepo], query: &str) -> Option<&'a repo_manager::ManagedRepo> {
    let q = query.to_lowercase();
    // Exact owner/name
    repos.iter().find(|r| format!("{}/{}", r.owner, r.name).to_lowercase() == q)
        // Exact name
        .or_else(|| repos.iter().find(|r| r.name.to_lowercase() == q))
        // Partial
        .or_else(|| repos.iter().find(|r| r.name.to_lowercase().contains(&q)))
}

fn suggest_repos(repos: &[repo_manager::ManagedRepo], query: &str) {
    let q = query.to_lowercase();
    let matches: Vec<_> = repos.iter()
        .filter(|r| r.name.to_lowercase().contains(&q) || r.owner.to_lowercase().contains(&q))
        .take(5)
        .collect();
    if !matches.is_empty() {
        eprintln!("  Did you mean:");
        for r in matches {
            eprintln!("    spark tag add {}/{} <tag>", r.owner, r.name);
        }
    }
}
