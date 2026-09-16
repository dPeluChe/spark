//! `spark pull` — pull repos by name, `all`, or `--tag`.

use super::select_repos;
use crate::config;
use crate::scanner;
use scanner::repo_manager::{ManagedRepo, RepoStatus};
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn cmd_pull(query: &str, tag: Option<String>, config: &config::SparkConfig) {
    let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);

    let filtered: Vec<_> = if let Some(ref tag_name) = tag {
        let by_tag = select_repos(&repos, None, Some(tag_name));
        if by_tag.is_empty() {
            eprintln!("  No repos with tag '{}'", tag_name);
            std::process::exit(1);
        }
        println!("  Tag: {}", tag_name);
        by_tag
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
            select_repos(&repos, Some(&q), None)
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
    let summary = pull_all(&filtered);

    if summary.pulled > 0 {
        println!("  {} repos pulled", summary.pulled);
    }
    if summary.up_to_date > 0 {
        println!("  {} repos already up to date", summary.up_to_date);
    }
    for (name, status) in &summary.skipped {
        println!("  - {} ({})", name, status);
    }
    for (name, err) in &summary.errors {
        let line = err
            .lines()
            .find(|l| l.starts_with("fatal:") || l.starts_with("error:"))
            .or_else(|| err.lines().next())
            .unwrap_or(err);
        match error_hint(err) {
            Some(hint) => eprintln!("  x {}: {} [{}]", name, line, hint),
            None => eprintln!("  x {}: {}", name, line),
        }
    }
    if summary.pulled == 0 && summary.skipped.is_empty() && summary.errors.is_empty() {
        println!("  All repos up to date");
    }
}

/// One-line remediation hint for common pull/check failures
fn error_hint(err: &str) -> Option<&'static str> {
    let e = err.to_lowercase();
    if (e.contains("repository") && e.contains("not found"))
        || e.contains("does not appear to be a git repository")
    {
        Some("remote gone — remove with: spark rm <name>")
    } else if e.contains("case-insensitive") || e.contains("reftable") {
        Some("ref casing conflict — fix: git refs migrate --ref-format=reftable")
    } else if e.contains("could not be updated") {
        Some("stale refs — fix: git remote prune origin")
    } else if e.contains("not possible to fast-forward") || e.contains("diverging branches") {
        Some("diverged — manual merge or rebase needed")
    } else {
        None
    }
}

enum PullOutcome {
    Pulled,
    Skipped(RepoStatus),
    Failed(String),
}

struct PullSummary {
    pulled: usize,
    up_to_date: usize,
    skipped: Vec<(String, RepoStatus)>,
    errors: Vec<(String, String)>,
}

fn pull_all(filtered: &[&ManagedRepo]) -> PullSummary {
    let total = filtered.len();
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let (tx, rx) = std::sync::mpsc::channel::<(usize, PullOutcome)>();

    std::thread::scope(|s| {
        let next = &next;
        let done = &done;
        for _ in 0..scanner::repo_manager::STATUS_CONCURRENCY.min(total) {
            let tx = tx.clone();
            s.spawn(move || loop {
                let i = next.fetch_add(1, Ordering::Relaxed);
                if i >= total {
                    break;
                }
                let repo = filtered[i];
                // check_repo_status already fetched (with --prune); merge only
                // when strictly behind so we never fetch twice.
                let outcome = match scanner::repo_manager::check_repo_status(&repo.path) {
                    RepoStatus::Behind(_) => {
                        match scanner::repo_manager::merge_ff_only(&repo.path) {
                            Ok(_) => PullOutcome::Pulled,
                            Err(e) => PullOutcome::Failed(e),
                        }
                    }
                    status => PullOutcome::Skipped(status),
                };
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                eprint!("\r  [{}/{}] {}/{}", n, total, repo.owner, repo.name);
                let _ = tx.send((i, outcome));
            });
        }
    });
    drop(tx);
    eprintln!("\r{}\r", " ".repeat(60));

    let mut summary = PullSummary {
        pulled: 0,
        up_to_date: 0,
        skipped: Vec::new(),
        errors: Vec::new(),
    };
    let mut cache_updates = Vec::new();

    for (i, outcome) in rx {
        let repo = filtered[i];
        let key = repo.path.display().to_string();
        let name = format!("{}/{}", repo.owner, repo.name);
        match outcome {
            PullOutcome::Pulled => {
                summary.pulled += 1;
                cache_updates.push((key, "up_to_date".to_string()));
            }
            PullOutcome::Skipped(RepoStatus::UpToDate) => {
                summary.up_to_date += 1;
                cache_updates.push((key, "up_to_date".to_string()));
            }
            PullOutcome::Skipped(status) => {
                summary.skipped.push((name, status.clone()));
                cache_updates.push((key, scanner::repo_manager::status_to_string(&status)));
            }
            PullOutcome::Failed(e) => summary.errors.push((name, e)),
        }
    }
    scanner::repo_manager::save_statuses_to_cache(cache_updates);
    summary
}

#[cfg(test)]
mod tests {
    use super::error_hint;

    #[test]
    fn test_error_hint() {
        assert!(error_hint("fatal: repository 'x' not found")
            .unwrap()
            .contains("spark rm"));
        assert!(error_hint("error: You're on a case-insensitive filesystem")
            .unwrap()
            .contains("reftable"));
        assert!(error_hint("some local refs could not be updated")
            .unwrap()
            .contains("prune"));
        assert!(error_hint("fatal: Not possible to fast-forward, aborting.")
            .unwrap()
            .contains("diverged"));
        assert!(error_hint("ssh: connect to host x: Operation timed out").is_none());
    }
}
