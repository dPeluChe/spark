//! JSON output contracts for `--json` flags (`json_version: 1`).
//!
//! These shapes are the stable API for agents and CI — see docs/dev/ROADMAP.md.
//! Breaking changes require a `JSON_VERSION` bump.

use crate::scanner::repo_manager::{ManagedRepo, RepoStatus};
use serde::{Deserialize, Serialize};

pub const JSON_VERSION: u8 = 1;

/// RFC3339 UTC timestamp (seconds precision) for `generated_at` fields.
pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Print a JSON contract to stdout.
pub fn print<T: Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).unwrap_or_default()
    );
}

/// Map a `RepoStatus` to (kind, ahead, behind, dirty). `dirty` is orthogonal
/// to `kind` so consumers can filter either dimension.
pub fn status_kind(status: &RepoStatus) -> (&'static str, usize, usize, bool) {
    match status {
        RepoStatus::UpToDate => ("up_to_date", 0, 0, false),
        RepoStatus::Behind(n) => ("behind", 0, *n, false),
        RepoStatus::Ahead(n) => ("ahead", *n, 0, false),
        RepoStatus::Diverged { ahead, behind } => ("diverged", *ahead, *behind, false),
        RepoStatus::Dirty { ahead, behind } => ("dirty", *ahead, *behind, true),
        RepoStatus::Error(_) => ("error", 0, 0, false),
        RepoStatus::Checking => ("checking", 0, 0, false),
    }
}

// ─── status ───

#[derive(Serialize)]
pub struct StatusRepo {
    pub host: String,
    pub owner: String,
    pub name: String,
    pub path: String,
    pub branch: String,
    pub status: &'static str,
    pub ahead: usize,
    pub behind: usize,
    pub dirty: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub last_commit: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Serialize, Default)]
pub struct StatusSummary {
    pub total: usize,
    pub up_to_date: usize,
    pub behind: usize,
    pub ahead: usize,
    pub dirty: usize,
    pub diverged: usize,
    pub error: usize,
    pub checking: usize,
}

#[derive(Serialize)]
pub struct StatusJson {
    pub json_version: u8,
    pub generated_at: String,
    pub summary: StatusSummary,
    pub repos: Vec<StatusRepo>,
}

/// Summary counters for status-style output, bucketed by status kind.
pub fn summarize_statuses(statuses: &[(&ManagedRepo, RepoStatus)]) -> StatusSummary {
    let mut s = StatusSummary::default();
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

// ─── list ───

#[derive(Serialize)]
pub struct ListRepo {
    pub host: String,
    pub owner: String,
    pub name: String,
    pub path: String,
    pub branch: String,
    pub last_commit: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Serialize)]
pub struct ListJson {
    pub json_version: u8,
    pub repos: Vec<ListRepo>,
}

// ─── ps ───

#[derive(Serialize)]
pub struct PortJson {
    pub port: u16,
    pub pid: u32,
    pub process: String,
    pub runtime: String,
    pub project: Option<String>,
    /// dev | system | service | app
    pub kind: &'static str,
}

#[derive(Serialize)]
pub struct PsPortsJson {
    pub json_version: u8,
    pub ports: Vec<PortJson>,
}

#[derive(Serialize)]
pub struct ProcessJson {
    pub pid: u32,
    pub cpu: String,
    pub mem: String,
    pub name: String,
    pub command: String,
    pub ports: Vec<u16>,
}

#[derive(Serialize)]
pub struct PsProcessesJson {
    pub json_version: u8,
    pub query: String,
    pub processes: Vec<ProcessJson>,
}

// ─── audit ───

#[derive(Serialize)]
pub struct AuditSummaryJson {
    pub total: usize,
    pub secrets: usize,
    pub history: usize,
    pub patterns: usize,
    pub deps: usize,
    pub npm_audit: bool,
}

#[derive(Serialize)]
pub struct AuditJson {
    pub json_version: u8,
    pub path: String,
    pub generated_at: String,
    pub summary: AuditSummaryJson,
    pub secrets: Vec<crate::scanner::secret_scanner::AuditResult>,
    pub history: Vec<crate::scanner::history_scanner::HistoryFinding>,
    pub patterns: Vec<crate::scanner::code_patterns::PatternFinding>,
    pub deps: Option<crate::scanner::dep_scanner::DepScanResult>,
}

// ─── report ───

#[derive(Serialize)]
pub struct ReportDiskRepo {
    pub repo: String,
    pub bytes: u64,
}

#[derive(Serialize)]
pub struct ReportDisk {
    pub artifacts_bytes: u64,
    pub artifact_repos: usize,
    pub top_repos: Vec<ReportDiskRepo>,
    pub system_bytes: u64,
    pub system_items: usize,
}

#[derive(Serialize)]
pub struct ReportTools {
    /// true when `--fresh` ran the version checks; false = not checked
    pub checked: bool,
    pub outdated: usize,
}

/// Summary persisted by `spark audit` (feeds `spark report`).
#[derive(Serialize, Deserialize)]
pub struct ReportSecurity {
    pub generated_at: String,
    pub path: String,
    pub total: usize,
}

#[derive(Serialize)]
pub struct ReportJson {
    pub json_version: u8,
    /// Running SPARK version (compile-time package version)
    pub spark_version: String,
    pub generated_at: String,
    pub repos: StatusSummary,
    pub disk: ReportDisk,
    pub ports: Vec<PortJson>,
    pub tools: ReportTools,
    pub security: Option<ReportSecurity>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_kind_mapping() {
        assert_eq!(
            status_kind(&RepoStatus::UpToDate),
            ("up_to_date", 0, 0, false)
        );
        assert_eq!(status_kind(&RepoStatus::Behind(3)), ("behind", 0, 3, false));
        assert_eq!(status_kind(&RepoStatus::Ahead(2)), ("ahead", 2, 0, false));
        assert_eq!(
            status_kind(&RepoStatus::Diverged {
                ahead: 1,
                behind: 4
            }),
            ("diverged", 1, 4, false)
        );
        assert_eq!(
            status_kind(&RepoStatus::Dirty {
                ahead: 0,
                behind: 2
            }),
            ("dirty", 0, 2, true)
        );
        assert_eq!(
            status_kind(&RepoStatus::Error("x".into())),
            ("error", 0, 0, false)
        );
        assert_eq!(
            status_kind(&RepoStatus::Checking),
            ("checking", 0, 0, false)
        );
    }

    #[test]
    fn test_summarize_statuses() {
        let r = |host: &str| ManagedRepo {
            path: std::path::PathBuf::from(format!("/tmp/{host}")),
            name: "r".into(),
            remote_url: String::new(),
            branch: "main".into(),
            status: RepoStatus::Checking,
            host: host.into(),
            owner: "o".into(),
            last_commit: None,
            size: 0,
        };
        let a = r("a");
        let b = r("b");
        let c = r("c");
        let statuses = vec![
            (&a, RepoStatus::UpToDate),
            (&b, RepoStatus::Behind(3)),
            (
                &c,
                RepoStatus::Dirty {
                    ahead: 1,
                    behind: 2,
                },
            ),
        ];
        let s = summarize_statuses(&statuses);
        assert_eq!(s.total, 3);
        assert_eq!(s.up_to_date, 1);
        assert_eq!(s.behind, 1);
        assert_eq!(s.dirty, 1);
    }

    #[test]
    fn test_report_json_shape() {
        let value = ReportJson {
            json_version: JSON_VERSION,
            spark_version: "0.5.1".into(),
            generated_at: "2026-09-16T00:00:00Z".into(),
            repos: StatusSummary {
                total: 2,
                behind: 1,
                ..Default::default()
            },
            disk: ReportDisk {
                artifacts_bytes: 1024,
                artifact_repos: 1,
                top_repos: vec![ReportDiskRepo {
                    repo: "o/r".into(),
                    bytes: 1024,
                }],
                system_bytes: 2048,
                system_items: 3,
            },
            ports: vec![],
            tools: ReportTools {
                checked: false,
                outdated: 0,
            },
            security: Some(ReportSecurity {
                generated_at: "2026-09-15T00:00:00Z".into(),
                path: "/tmp".into(),
                total: 2,
            }),
        };
        let json: serde_json::Value = serde_json::to_value(&value).unwrap();
        assert_eq!(json["json_version"], 1);
        assert_eq!(json["spark_version"], "0.5.1");
        assert_eq!(json["repos"]["behind"], 1);
        assert_eq!(json["disk"]["top_repos"][0]["repo"], "o/r");
        assert_eq!(json["tools"]["checked"], false);
        assert_eq!(json["security"]["total"], 2);
        // round-trip the persisted audit summary
        let back: ReportSecurity = serde_json::from_value(json["security"].clone()).unwrap();
        assert_eq!(back.path, "/tmp");
    }

    #[test]
    fn test_status_json_shape() {
        let value = StatusJson {
            json_version: JSON_VERSION,
            generated_at: "2026-09-16T00:00:00Z".into(),
            summary: StatusSummary {
                total: 1,
                behind: 1,
                ..Default::default()
            },
            repos: vec![StatusRepo {
                host: "github.com".into(),
                owner: "dPeluChe".into(),
                name: "spark".into(),
                path: "/tmp/spark".into(),
                branch: "main".into(),
                status: "behind",
                ahead: 0,
                behind: 3,
                dirty: false,
                error: None,
                last_commit: Some("2d ago".into()),
                tags: vec!["work".into()],
            }],
        };
        let json: serde_json::Value = serde_json::to_value(&value).unwrap();
        assert_eq!(json["json_version"], 1);
        assert_eq!(json["summary"]["behind"], 1);
        assert_eq!(json["repos"][0]["status"], "behind");
        assert_eq!(json["repos"][0]["behind"], 3);
        assert_eq!(json["repos"][0]["tags"][0], "work");
        assert!(json["repos"][0].get("error").is_none());
    }
}
