//! `spark status` — show which repos need pull, with optional tag filter.

use super::select_repos;
use crate::cli::json;
use crate::config;
use crate::scanner;
use scanner::repo_manager::RepoStatus;

pub fn cmd_status(
    query: Option<String>,
    tag: Option<String>,
    exit_code: bool,
    json_out: bool,
    config: &config::SparkConfig,
) {
    let repos = scanner::repo_manager::list_managed_repos_lite(&config.repos_root);
    let filtered = select_repos(&repos, query.as_deref(), tag.as_deref());

    if filtered.is_empty() {
        if json_out {
            json::print(&json::StatusJson {
                json_version: json::JSON_VERSION,
                generated_at: json::now_iso(),
                summary: json::StatusSummary::default(),
                repos: Vec::new(),
            });
        } else if let Some(t) = &tag {
            println!("  No repos with tag '{}'", t);
        } else {
            println!("  No repos found");
        }
        return;
    }

    if let Some(t) = &tag {
        if !json_out {
            println!("  Tag: {}", t);
        }
    }
    if !json_out {
        println!("  Checking {} repos...\n", filtered.len());
    }

    let statuses = fetch_statuses(&filtered);

    if json_out {
        let tags = scanner::repo_tags::load_tags();
        let repos_json: Vec<json::StatusRepo> = statuses
            .iter()
            .map(|(repo, status)| {
                let key = scanner::repo_tags::repo_key(&repo.host, &repo.owner, &repo.name);
                let (kind, ahead, behind, dirty) = json::status_kind(status);
                json::StatusRepo {
                    host: repo.host.clone(),
                    owner: repo.owner.clone(),
                    name: repo.name.clone(),
                    path: repo.path.display().to_string(),
                    branch: repo.branch.clone(),
                    status: kind,
                    ahead,
                    behind,
                    dirty,
                    error: match status {
                        RepoStatus::Error(e) => Some(e.clone()),
                        _ => None,
                    },
                    last_commit: repo.last_commit.clone(),
                    tags: tags.tags_for_repo(&key),
                }
            })
            .collect();
        json::print(&json::StatusJson {
            json_version: json::JSON_VERSION,
            generated_at: json::now_iso(),
            summary: summarize(&statuses),
            repos: repos_json,
        });
    } else {
        print_status_table(&statuses);
        print_summary(&statuses);

        let all_tags = scanner::repo_tags::load_tags().all_tags();
        if !all_tags.is_empty() {
            println!("\n  \x1b[90mTags: {}\x1b[0m", all_tags.join(", "));
        }
        println!("  \x1b[90mspark tag add <repo> <tag>    add tag to a repo\x1b[0m");
        println!("  \x1b[90mspark tag list               see all tags\x1b[0m");
    }

    // CI/agent gate: anything not up to date counts as needing attention
    if exit_code {
        let needs = statuses
            .iter()
            .filter(|(_, s)| !matches!(s, RepoStatus::UpToDate))
            .count();
        if needs > 0 {
            std::process::exit(1);
        }
    }
}

/// Summary counters for `--json`, bucketed by status kind.
fn summarize(
    statuses: &[(&scanner::repo_manager::ManagedRepo, RepoStatus)],
) -> json::StatusSummary {
    let mut s = json::StatusSummary::default();
    for (_, status) in statuses {
        s.total += 1;
        match status {
            RepoStatus::UpToDate => s.up_to_date += 1,
            RepoStatus::Behind(_) => s.behind += 1,
            RepoStatus::Ahead(_) => s.ahead += 1,
            RepoStatus::Diverged { .. } => s.diverged += 1,
            RepoStatus::Dirty { .. } => s.dirty += 1,
            RepoStatus::Error(_) => s.error += 1,
            RepoStatus::Checking => s.checking += 1,
        }
    }
    s
}

fn fetch_statuses<'a>(
    filtered: &[&'a scanner::repo_manager::ManagedRepo],
) -> Vec<(
    &'a scanner::repo_manager::ManagedRepo,
    scanner::repo_manager::RepoStatus,
)> {
    let cache = scanner::repo_manager::load_status_cache();
    let mut statuses = Vec::with_capacity(filtered.len());
    let mut to_check = Vec::new();

    for repo in filtered {
        let key = repo.path.display().to_string();
        match cache
            .get(&key)
            .filter(|(_, ts)| scanner::repo_manager::is_cache_valid(*ts))
        {
            Some((s, _)) => statuses.push((*repo, scanner::repo_manager::string_to_status(s))),
            None => to_check.push(*repo),
        }
    }

    if !to_check.is_empty() {
        let fresh = scanner::repo_manager::check_statuses_parallel(&to_check);
        scanner::repo_manager::save_statuses_to_cache(to_check.iter().zip(&fresh).map(|(r, s)| {
            (
                r.path.display().to_string(),
                scanner::repo_manager::status_to_string(s),
            )
        }));
        statuses.extend(to_check.into_iter().zip(fresh));
    }

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
        scanner::repo_manager::RepoStatus::Dirty { .. } => "*",
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
