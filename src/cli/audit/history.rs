//! Render the Git History section of the audit report.

use crate::scanner;
use std::fmt::Write as FmtWrite;

pub(super) fn render(report: &mut String, history: &[scanner::history_scanner::HistoryFinding]) {
    if history.is_empty() {
        return;
    }

    println!("  \x1b[1m--- Git History (past commits) ---\x1b[0m\n");
    let _ = writeln!(report, "--- Git History (past commits) ---\n");
    for hf in history {
        let rel_path = hf
            .finding
            .file_path
            .strip_prefix(&hf.finding.project_path)
            .unwrap_or(&hf.finding.file_path);
        println!(
            "    \x1b[31m!!\x1b[0m  {} — {} \x1b[90m({})\x1b[0m",
            rel_path.display(),
            hf.finding.description,
            hf.finding.redacted_match
        );
        println!(
            "        \x1b[90mcommit {} by {} on {} — {}\x1b[0m",
            hf.commit_sha, hf.author, hf.date, hf.commit_msg
        );
        let _ = writeln!(
            report,
            "    !!  {} — {} ({})",
            rel_path.display(),
            hf.finding.description,
            hf.finding.redacted_match
        );
        let _ = writeln!(
            report,
            "        commit {} by {} on {} — {}",
            hf.commit_sha, hf.author, hf.date, hf.commit_msg
        );
    }
    println!(
        "\n  \x1b[31m{} secrets found in git history\x1b[0m",
        history.len()
    );
    println!("  \x1b[90mThese may still be in the repo even if files were deleted.\x1b[0m");
    println!("  \x1b[90mConsider rotating these credentials.\x1b[0m\n");
    let _ = writeln!(
        report,
        "\n  {} secrets in git history — consider rotating credentials.\n",
        history.len()
    );
}
