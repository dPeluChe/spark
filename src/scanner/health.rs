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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_health() {
        let now = Utc::now();
        let modified = std::time::SystemTime::now();
        let (score, grade) = calculate_health(Some(now), Some(modified), true, false, 0);
        assert_eq!(score, 100);
        assert_eq!(grade, HealthGrade::A);
    }

    #[test]
    fn test_no_remote_penalty() {
        let now = Utc::now();
        let modified = std::time::SystemTime::now();
        let (score, _) = calculate_health(Some(now), Some(modified), false, false, 0);
        assert_eq!(score, 85); // -15 for no remote
    }

    #[test]
    fn test_no_commits_penalty() {
        let modified = std::time::SystemTime::now();
        let (score, _) = calculate_health(None, Some(modified), true, false, 0);
        assert_eq!(score, 70); // -30 for no commits
    }

    #[test]
    fn test_large_artifacts_penalty() {
        let now = Utc::now();
        let modified = std::time::SystemTime::now();
        let artifact_200mb = 200 * 1024 * 1024;
        let (score, _) = calculate_health(Some(now), Some(modified), true, false, artifact_200mb);
        assert_eq!(score, 80); // -20 for >100MB artifacts
    }

    #[test]
    fn test_worst_case_scores_low() {
        let old_commit = DateTime::from_timestamp(0, 0);
        // epoch commit (-30), no remote (-15), dirty+stale (-10), >100MB artifacts (-20) = 25
        let (score, grade) = calculate_health(old_commit, None, false, true, 500 * 1024 * 1024);
        assert_eq!(score, 25);
        assert_eq!(grade, HealthGrade::D);
    }

    #[test]
    fn test_grade_boundaries() {
        let modified = std::time::SystemTime::now();

        // No commits (-30) = score 70 -> Grade B: 60-79
        let (_, grade) = calculate_health(None, Some(modified), true, false, 0);
        assert_eq!(grade, HealthGrade::B);
    }
}
