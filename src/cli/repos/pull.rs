//! `spark pull` — pull repos by name, `all`, or `--tag`.

use super::select_repos;
use crate::config;
use crate::scanner;
use scanner::repo_manager::{ManagedRepo, RepoStatus};
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn cmd_pull(query: &str, tag: Option<String>, config: &config::SparkConfig) {
    let repos = scanner::repo_manager::list_managed_repos_lite(&config.repos_root);

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
        match status {
            // Fetch/check failures arrive as statuses too — condense them like
            // merge failures instead of dumping multi-line stderr
            RepoStatus::Error(e) => match error_hint(e) {
                Some(hint) => eprintln!("  x {}: {} [{}]", name, err_line(e), hint),
                None => eprintln!("  x {}: {}", name, err_line(e)),
            },
            s => println!("  - {} ({})", name, s),
        }
    }
    for (name, err) in &summary.errors {
        match error_hint(err) {
            Some(hint) => eprintln!("  x {}: {} [{}]", name, err_line(err), hint),
            None => eprintln!("  x {}: {}", name, err_line(err)),
        }
    }

    render_resolve(&summary);

    if summary.pulled == 0 && summary.skipped.is_empty() && summary.errors.is_empty() {
        println!("  All repos up to date");
    }
}

/// First `fatal:`/`error:` line of a git failure (falls back to the first line)
fn err_line(err: &str) -> &str {
    err.lines()
        .find(|l| l.starts_with("fatal:") || l.starts_with("error:"))
        .or_else(|| err.lines().next())
        .unwrap_or(err)
}

/// Exact commands to fix what pull could not touch, one per repo.
fn render_resolve(summary: &PullSummary) {
    let mut lines: Vec<String> = Vec::new();
    for (name, status) in &summary.skipped {
        if let Some(l) = resolve_hint(name, status) {
            lines.push(l);
        }
    }
    for (name, err) in &summary.errors {
        if let Some(l) = error_resolve(name, err) {
            lines.push(l);
        }
    }
    if lines.is_empty() {
        return;
    }
    println!("\n  To resolve:");
    for l in lines {
        println!("    {}", l);
    }
}

/// Remediation command for a skipped repo, by status kind.
fn resolve_hint(name: &str, status: &RepoStatus) -> Option<String> {
    match status {
        RepoStatus::Dirty { .. } => Some(format!(
            "spark cd {name}    # commit or stash, then: spark pull {name}"
        )),
        RepoStatus::Diverged { .. } => Some(format!(
            "spark cd {name}    # diverged: merge or rebase onto upstream"
        )),
        RepoStatus::Ahead(_) => Some(format!(
            "spark cd {name}    # unpushed commits — push when ready"
        )),
        RepoStatus::Error(e) => error_resolve(name, e),
        _ => None,
    }
}

/// Remediation command for an error, when the hint points at one.
fn error_resolve(name: &str, err: &str) -> Option<String> {
    let hint = error_hint(err)?;
    if hint.contains("remote gone") {
        Some(format!(
            "spark rm {name}    # remote gone — remove the local clone"
        ))
    } else if hint.contains("reftable") {
        Some(format!(
            "git -C \"$(spark cd {name})\" refs migrate --ref-format=reftable"
        ))
    } else if hint.contains("prune") {
        Some(format!("git -C \"$(spark cd {name})\" remote prune origin"))
    } else {
        None
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
    } else if e.contains("could not be updated")
        || e.contains("incorrect old value")
        || e.contains("refname conflict")
    {
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
                crate::utils::shell::progress_line(
                    n,
                    total,
                    &format!("{}/{}", repo.owner, repo.name),
                );
                let _ = tx.send((i, outcome));
            });
        }
    });
    drop(tx);
    crate::utils::shell::clear_progress_line();

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
    use super::{error_hint, error_resolve, resolve_hint};
    use crate::scanner::repo_manager::RepoStatus;

    #[test]
    fn test_resolve_hint() {
        let dirty = resolve_hint(
            "o/r",
            &RepoStatus::Dirty {
                ahead: 0,
                behind: 3,
            },
        )
        .unwrap();
        assert!(dirty.starts_with("spark cd o/r"));
        assert!(dirty.contains("spark pull o/r"));

        let diverged = resolve_hint(
            "o/r",
            &RepoStatus::Diverged {
                ahead: 1,
                behind: 2,
            },
        )
        .unwrap();
        assert!(diverged.contains("merge or rebase"));

        let ahead = resolve_hint("o/r", &RepoStatus::Ahead(2)).unwrap();
        assert!(ahead.contains("unpushed"));

        assert!(resolve_hint("o/r", &RepoStatus::UpToDate).is_none());
    }

    #[test]
    fn test_error_resolve() {
        let gone = error_resolve("o/r", "fatal: repository 'x' not found").unwrap();
        assert!(gone.starts_with("spark rm o/r"));

        let case_conflict = error_resolve(
            "o/r",
            "error: You're on a case-insensitive filesystem, and the remote...",
        )
        .unwrap();
        assert!(case_conflict.contains("refs migrate"));

        let stale = error_resolve(
            "o/r",
            "fetch failed: error: fetching ref refs/remotes/origin/b failed: incorrect old value provided",
        )
        .unwrap();
        assert!(stale.contains("remote prune origin"));

        assert!(error_resolve("o/r", "ssh: connect timed out").is_none());
    }

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
