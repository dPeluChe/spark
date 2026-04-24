//! `spark status` — show which repos need pull, with optional tag filter.

use super::super::filter_repo;
use crate::config;
use crate::scanner;

pub fn cmd_status(query: Option<String>, tag: Option<String>, config: &config::SparkConfig) {
    let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
    let tag_filter = tag.as_ref().map(|t| {
        let tags = scanner::repo_tags::load_tags();
        tags.repos_for_tag(t)
    });

    let filtered: Vec<_> = if let Some(ref tag_repos) = tag_filter {
        repos
            .iter()
            .filter(|r| {
                let key = scanner::repo_tags::repo_key(&r.host, &r.owner, &r.name);
                tag_repos.contains(&key)
            })
            .collect()
    } else {
        match &query {
            Some(q) => {
                let q = q.to_lowercase();
                repos.iter().filter(|r| filter_repo(r, &q)).collect()
            }
            None => repos.iter().collect(),
        }
    };

    if let Some(t) = &tag {
        if filtered.is_empty() {
            println!("  No repos with tag '{}'", t);
            return;
        }
        println!("  Tag: {}", t);
    }

    if filtered.is_empty() {
        println!("  No repos found");
        return;
    }
    println!("  Checking {} repos...\n", filtered.len());

    let statuses = fetch_statuses(&filtered);
    print_status_table(&statuses);
    print_summary(&statuses);

    let all_tags = scanner::repo_tags::load_tags().all_tags();
    if !all_tags.is_empty() {
        println!("\n  \x1b[90mTags: {}\x1b[0m", all_tags.join(", "));
    }
    println!("  \x1b[90mspark tag add <repo> <tag>    add tag to a repo\x1b[0m");
    println!("  \x1b[90mspark tag list               see all tags\x1b[0m");
}

fn fetch_statuses<'a>(
    filtered: &[&'a scanner::repo_manager::ManagedRepo],
) -> Vec<(
    &'a scanner::repo_manager::ManagedRepo,
    scanner::repo_manager::RepoStatus,
)> {
    let cache = scanner::repo_manager::load_status_cache();
    let mut statuses = Vec::with_capacity(filtered.len());

    for (i, repo) in filtered.iter().enumerate() {
        eprint!(
            "\r  [{}/{}] {}/{}",
            i + 1,
            filtered.len(),
            repo.owner,
            repo.name
        );
        let key = repo.path.display().to_string();
        let status = cache
            .get(&key)
            .filter(|(_, ts)| scanner::repo_manager::is_cache_valid(*ts))
            .map(|(s, _)| scanner::repo_manager::string_to_status(s))
            .unwrap_or_else(|| {
                let s = scanner::repo_manager::check_repo_status(&repo.path);
                scanner::repo_manager::save_status_to_cache(
                    &key,
                    &scanner::repo_manager::status_to_string(&s),
                );
                s
            });
        statuses.push((*repo, status));
    }
    eprintln!("\r{}\r", " ".repeat(60));

    // Up-to-date alphabetic first, then outdated alphabetic
    statuses.sort_by(|a, b| {
        let a_ok = matches!(a.1, scanner::repo_manager::RepoStatus::UpToDate);
        let b_ok = matches!(b.1, scanner::repo_manager::RepoStatus::UpToDate);
        b_ok.cmp(&a_ok)
            .then(a.0.owner.to_lowercase().cmp(&b.0.owner.to_lowercase()))
            .then(a.0.name.to_lowercase().cmp(&b.0.name.to_lowercase()))
    });
    statuses
}

fn print_status_table(
    statuses: &[(
        &scanner::repo_manager::ManagedRepo,
        scanner::repo_manager::RepoStatus,
    )],
) {
    let tags = scanner::repo_tags::load_tags();
    let max_name = statuses
        .iter()
        .map(|(r, _)| r.owner.len() + 1 + r.name.len())
        .max()
        .unwrap_or(20)
        + 2;

    let needs_attention: Vec<_> = statuses
        .iter()
        .filter(|(_, s)| !matches!(s, scanner::repo_manager::RepoStatus::UpToDate))
        .collect();
    let up_to_date: Vec<_> = statuses
        .iter()
        .filter(|(_, s)| matches!(s, scanner::repo_manager::RepoStatus::UpToDate))
        .collect();

    if !needs_attention.is_empty() {
        println!(
            "  \x1b[33mNeeds attention ({})\x1b[0m\n",
            needs_attention.len()
        );
        for (repo, status) in &needs_attention {
            let key = scanner::repo_tags::repo_key(&repo.host, &repo.owner, &repo.name);
            print_status_row(repo, status, max_name, &tags.tags_for_repo(&key));
        }
        println!();
    }

    if !up_to_date.is_empty() {
        println!("  \x1b[32mUp to date ({})\x1b[0m\n", up_to_date.len());
        for (repo, status) in &up_to_date {
            let key = scanner::repo_tags::repo_key(&repo.host, &repo.owner, &repo.name);
            print_status_row(repo, status, max_name, &tags.tags_for_repo(&key));
        }
    }
}

fn print_status_row(
    repo: &scanner::repo_manager::ManagedRepo,
    status: &scanner::repo_manager::RepoStatus,
    max_name: usize,
    repo_tags: &[String],
) {
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
    let tag_str = if repo_tags.is_empty() {
        String::new()
    } else {
        format!("  \x1b[36m[{}]\x1b[0m", repo_tags.join(","))
    };

    println!(
        "  {:<width$}   {}   {:<14}  {}{}",
        repo_name,
        indicator,
        status_str,
        age,
        tag_str,
        width = max_name
    );
}

fn print_summary(
    statuses: &[(
        &scanner::repo_manager::ManagedRepo,
        scanner::repo_manager::RepoStatus,
    )],
) {
    let needs = statuses
        .iter()
        .filter(|(_, s)| !matches!(s, scanner::repo_manager::RepoStatus::UpToDate))
        .count();
    let updated = statuses
        .iter()
        .filter(|(_, s)| matches!(s, scanner::repo_manager::RepoStatus::UpToDate))
        .count();
    println!(
        "\n  {} total — {} need pull, {} up to date",
        statuses.len(),
        needs,
        updated
    );
    if needs > 0 {
        println!("  spark pull <name>          pull a specific repo");
        println!("  spark pull all             pull all behind repos");
        println!("  spark pull all --tag <t>   pull repos by tag");
    }
}
