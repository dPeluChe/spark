use chrono::{DateTime, Utc};
use crate::scanner::repo_scanner::HealthGrade;

/// Calculate health score (0-100) and grade for a repository
pub fn calculate_health(
    last_commit: Option<DateTime<Utc>>,
    last_modified: Option<std::time::SystemTime>,
    has_remote: bool,
    is_dirty: bool,
    artifact_size: u64,
) -> (u8, HealthGrade) {
    let mut score: i32 = 100;

    // Last commit age: -5 per month, max -30
    if let Some(commit_date) = last_commit {
        let months_ago = (Utc::now() - commit_date).num_days() / 30;
        let penalty = (months_ago * 5).min(30) as i32;
        score -= penalty;
    } else {
        score -= 30; // No commits at all
    }

    // Last file modified: -3 per month, max -15
    if let Some(modified) = last_modified {
        if let Ok(duration) = modified.elapsed() {
            let months = duration.as_secs() / (30 * 24 * 3600);
            let penalty = (months * 3).min(15) as i32;
            score -= penalty;
        }
    }

    // No remote: -15
    if !has_remote {
        score -= 15;
    }

    // Dirty + stale: -10
    if is_dirty {
        if let Some(commit_date) = last_commit {
            let days_ago = (Utc::now() - commit_date).num_days();
            if days_ago > 30 {
                score -= 10;
            }
        }
    }

    // Large stale artifacts: scale up to -20
    let artifact_mb = artifact_size / (1024 * 1024);
    if artifact_mb > 100 {
        score -= 20;
    } else if artifact_mb > 50 {
        score -= 15;
    } else if artifact_mb > 10 {
        score -= 10;
    } else if artifact_mb > 1 {
        score -= 5;
    }

    let score = score.max(0).min(100) as u8;
    let grade = match score {
        80..=100 => HealthGrade::A,
        60..=79 => HealthGrade::B,
        40..=59 => HealthGrade::C,
        20..=39 => HealthGrade::D,
        _ => HealthGrade::F,
    };

    (score, grade)
}
