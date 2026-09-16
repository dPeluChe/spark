//! `spark report` — one-screen fleet status: repos, disk, ports, tools, security.
//!
//! Read-only triage view for humans and agents (see docs/dev/ROADMAP.md,
//! Phase 1). It never fixes anything; each section points at the command that does.

use super::json;
use super::repos::fetch_statuses;
use crate::config;
use crate::scanner;
use crate::scanner::port_scanner::{self, PortInfo};
use crate::utils::fs::format_size;
use std::sync::atomic::{AtomicUsize, Ordering};

pub fn cmd_report(fresh: bool, json_out: bool, config: &config::SparkConfig) {
    let repos = scanner::repo_manager::list_managed_repos_lite(&config.repos_root);
    let repo_refs: Vec<&scanner::repo_manager::ManagedRepo> = repos.iter().collect();

    if !json_out {
        println!(
            "  SPARK Fleet Report · v{} · {} repos · {}",
            env!("CARGO_PKG_VERSION"),
            repos.len(),
            if fresh {
                "fresh check"
            } else {
                "cached (--fresh re-fetches)"
            }
        );
    }

    // ── Repos ──
    let statuses = fetch_statuses(&repo_refs, !fresh);
    let repo_summary = json::summarize_statuses(&statuses);

    // ── Disk: artifacts across repos (parallel) + system cleanables ──
    let (artifacts_bytes, artifact_repos) = scan_artifacts_parallel(&repo_refs);
    let system_items = scanner::system_cleaner::scan_system();
    let system_bytes: u64 = system_items.iter().map(|i| i.size).sum();

    // ── Ports ──
    let all_ports = port_scanner::scan_ports();
    let dev_ports: Vec<&PortInfo> = all_ports
        .iter()
        .filter(|p| port_scanner::is_dev_server(p))
        .collect();

    // ── Tools (network; only with --fresh) ──
    let outdated_tools = if fresh {
        Some(count_outdated_tools())
    } else {
        None
    };

    // ── Security: last audit summary ──
    let security = last_audit();

    if json_out {
        json::print(&json::ReportJson {
            json_version: json::JSON_VERSION,
            spark_version: env!("CARGO_PKG_VERSION").to_string(),
            generated_at: json::now_iso(),
            repos: repo_summary,
            disk: json::ReportDisk {
                artifacts_bytes,
                artifact_repos: artifact_repos.len(),
                top_repos: artifact_repos
                    .iter()
                    .take(5)
                    .map(|(repo, bytes)| json::ReportDiskRepo {
                        repo: repo.clone(),
                        bytes: *bytes,
                    })
                    .collect(),
                system_bytes,
                system_items: system_items.len(),
            },
            ports: dev_ports
                .iter()
                .map(|p| json::PortJson {
                    port: p.port,
                    pid: p.pid,
                    process: p.process_name.clone(),
                    runtime: format!("{}", p.runtime),
                    project: p.project_dir.clone(),
                    kind: "dev",
                })
                .collect(),
            tools: json::ReportTools {
                checked: outdated_tools.is_some(),
                outdated: outdated_tools.unwrap_or(0),
            },
            security,
        });
        return;
    }

    render_human(
        &repo_summary,
        artifacts_bytes,
        &artifact_repos,
        system_bytes,
        system_items.len(),
        &dev_ports,
        outdated_tools,
        security.as_ref(),
    );
}

type ArtifactRepos = Vec<(String, u64)>;

/// Artifact bytes across repos, parallelized with the shared bounded pool.
fn scan_artifacts_parallel(repos: &[&scanner::repo_manager::ManagedRepo]) -> (u64, ArtifactRepos) {
    let total = repos.len();
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let (tx, rx) = std::sync::mpsc::channel::<(usize, u64)>();

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
                let repo = repos[i];
                let bytes: u64 = scanner::space_analyzer::find_artifacts(&repo.path)
                    .iter()
                    .map(|a| a.size)
                    .sum();
                let n = done.fetch_add(1, Ordering::Relaxed) + 1;
                eprint!("\r  [disk {}/{}] {}", n, total, repo.name);
                let _ = tx.send((i, bytes));
            });
        }
    });
    drop(tx);
    eprintln!("\r{}\r", " ".repeat(60));

    let mut per_repo = vec![0u64; total];
    for (i, bytes) in rx {
        per_repo[i] = bytes;
    }
    let mut ranked: ArtifactRepos = repos
        .iter()
        .zip(per_repo)
        .filter(|(_, bytes)| *bytes > 0)
        .map(|(repo, bytes)| (format!("{}/{}", repo.owner, repo.name), bytes))
        .collect();
    ranked.sort_by_key(|(_, bytes)| std::cmp::Reverse(*bytes));
    let total_bytes = ranked.iter().map(|(_, b)| b).sum();
    (total_bytes, ranked)
}

/// Outdated tool count via the updater detector (network-heavy).
fn count_outdated_tools() -> usize {
    use crate::updater::detector::Detector;
    use std::sync::Arc;

    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            eprintln!("  Checking tool versions...");
            let detector = Arc::new(Detector::new());
            detector.warm_up_cache().await;
            let tools = crate::core::inventory::get_inventory();
            let mut handles = Vec::with_capacity(tools.len());
            for tool in tools {
                let detector = Arc::clone(&detector);
                handles.push(tokio::spawn(async move {
                    let local = detector.get_local_version(&tool).await;
                    if local == "MISSING" {
                        return false;
                    }
                    let remote = detector.get_remote_version(&tool, &local).await;
                    remote != "Unknown" && remote != "Checking..." && remote != local
                }));
            }
            let mut count = 0;
            for h in handles {
                if h.await.unwrap_or(false) {
                    count += 1;
                }
            }
            count
        })
    })
}

/// Read the summary `spark audit` persists after each run.
fn last_audit() -> Option<json::ReportSecurity> {
    let path = dirs::config_dir()?.join("spark").join("last_audit.json");
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// "2d ago" style age for the audit timestamp.
fn audit_age(generated_at: &str) -> String {
    let Ok(ts) = chrono::DateTime::parse_from_rfc3339(generated_at) else {
        return "?".into();
    };
    let elapsed = (chrono::Utc::now() - ts.with_timezone(&chrono::Utc))
        .num_seconds()
        .max(0);
    if elapsed < 3600 {
        format!("{}m ago", (elapsed / 60).max(1))
    } else if elapsed < 86400 {
        format!("{}h ago", elapsed / 3600)
    } else {
        format!("{}d ago", elapsed / 86400)
    }
}

#[allow(clippy::too_many_arguments)]
fn render_human(
    repos: &json::StatusSummary,
    artifacts_bytes: u64,
    artifact_repos: &ArtifactRepos,
    system_bytes: u64,
    system_items: usize,
    dev_ports: &[&PortInfo],
    outdated_tools: Option<usize>,
    security: Option<&json::ReportSecurity>,
) {
    println!();
    // Repos
    println!(
        "  Repos       {} up to date · {} behind · {} dirty · {} diverged",
        repos.up_to_date, repos.behind, repos.dirty, repos.diverged
    );
    if repos.behind > 0 {
        println!("              → spark pull all  ({} behind)", repos.behind);
    }
    if repos.error > 0 {
        println!("              {} with errors → spark status", repos.error);
    }

    // Disk
    let recoverable = artifacts_bytes + system_bytes;
    println!("  Disk        {} recoverable", format_size(recoverable));
    if artifacts_bytes > 0 {
        let top = artifact_repos
            .first()
            .map(|(repo, bytes)| format!(" (top: {} {})", repo, format_size(*bytes)))
            .unwrap_or_default();
        println!(
            "              → {} artifacts in {} repos{}",
            format_size(artifacts_bytes),
            artifact_repos.len(),
            top
        );
    }
    if system_bytes > 0 {
        println!(
            "              → {} system in {} items (docker, caches, logs)",
            format_size(system_bytes),
            system_items
        );
    }
    if recoverable > 0 {
        println!("              → spark system  (TUI cleanup)");
    }

    // Ports
    if dev_ports.is_empty() {
        println!("  Ports       no dev servers");
    } else {
        let mut shown: Vec<String> = dev_ports
            .iter()
            .take(6)
            .map(|p| p.port.to_string())
            .collect();
        if dev_ports.len() > 6 {
            shown.push("…".into());
        }
        println!(
            "  Ports       {} dev {} ({})  → spark ps",
            dev_ports.len(),
            if dev_ports.len() == 1 {
                "server"
            } else {
                "servers"
            },
            shown.join(", ")
        );
    }

    // Tools
    match outdated_tools {
        Some(n) if n > 0 => println!("  Tools       {} outdated  → TUI Updater", n),
        Some(_) => println!("  Tools       all up to date"),
        None => println!("  Tools       not checked  (--fresh to check)"),
    }

    // Security
    match security {
        Some(s) if s.total > 0 => println!(
            "  Security    last audit {} · {} findings  → spark audit",
            audit_age(&s.generated_at),
            s.total
        ),
        Some(s) => println!(
            "  Security    last audit {} · clean  → spark audit",
            audit_age(&s.generated_at)
        ),
        None => println!("  Security    no audit yet  → spark audit"),
    }
    println!();
}
