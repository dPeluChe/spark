//! JSON output contracts for `--json` flags (`json_version: 1`).
//!
//! These shapes are the stable API for agents and CI — see docs/dev/ROADMAP.md.
//! Breaking changes require a `JSON_VERSION` bump.

use crate::scanner::repo_manager::RepoStatus;
use serde::Serialize;

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
