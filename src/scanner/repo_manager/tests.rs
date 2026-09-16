//! Tests for repo_manager — git URL parsing, status transitions, cache round-trip.

use super::meta::relative_age;
use super::*;

#[test]
fn test_parse_ssh_url() {
    let (host, owner, name) = parse_git_url("git@github.com:user/repo.git").unwrap();
    assert_eq!(host, "github.com");
    assert_eq!(owner, "user");
    assert_eq!(name, "repo");
}

#[test]
fn test_parse_https_url() {
    let (host, owner, name) = parse_git_url("https://github.com/user/repo.git").unwrap();
    assert_eq!(host, "github.com");
    assert_eq!(owner, "user");
    assert_eq!(name, "repo");
}

#[test]
fn test_relative_age() {
    let now = chrono::Utc::now().timestamp();
    assert_eq!(relative_age(now - 90), "1m ago");
    assert_eq!(relative_age(now - 7200), "2h ago");
    assert_eq!(relative_age(now - 86400 * 5), "5d ago");
    assert_eq!(relative_age(now - 86400 * 240), "8mo ago");
    assert_eq!(relative_age(now - 86400 * 800), "2y ago");
    assert_eq!(relative_age(now + 60), "1m ago"); // future timestamp clamps
}

#[test]
fn test_status_string_roundtrip() {
    for s in [
        RepoStatus::UpToDate,
        RepoStatus::Behind(3),
        RepoStatus::Ahead(2),
        RepoStatus::Diverged {
            ahead: 4,
            behind: 9,
        },
        RepoStatus::Dirty {
            ahead: 0,
            behind: 0,
        },
        RepoStatus::Dirty {
            ahead: 2,
            behind: 5,
        },
        RepoStatus::Error("boom".into()),
        RepoStatus::Checking,
    ] {
        assert_eq!(string_to_status(&status_to_string(&s)), s);
    }
    // legacy cache entries without counts still parse
    assert_eq!(
        string_to_status("dirty"),
        RepoStatus::Dirty {
            ahead: 0,
            behind: 0
        }
    );
}

/// Real git round-trip: clone a local bare origin, then drive it ahead and
/// confirm status flips Behind and merge_ff_only converges it.
#[test]
fn test_check_status_and_ff_merge() {
    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git")
            .args([
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "-c",
                "init.defaultBranch=main",
            ])
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(status.status.success(), "git {:?} failed", args);
    }

    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path();
    let origin = root.join("origin.git");
    let work = root.join("work");
    let other = root.join("other");

    git(root, &["init", "--bare", "origin.git"]);
    git(root, &["clone", "origin.git", "work"]);
    git(&work, &["commit", "--allow-empty", "-m", "one"]);
    git(&work, &["push", "-u", "origin", "main"]);

    assert_eq!(check_repo_status(&work), RepoStatus::UpToDate);

    git(&work, &["commit", "--allow-empty", "-m", "two"]);
    assert_eq!(check_repo_status(&work), RepoStatus::Ahead(1));

    git(root, &["clone", "origin.git", "other"]);
    git(&other, &["commit", "--allow-empty", "-m", "three"]);
    git(&other, &["push", "origin", "main"]);
    // work is now 1 ahead, 1 behind -> Diverged
    assert_eq!(
        check_repo_status(&work),
        RepoStatus::Diverged {
            ahead: 1,
            behind: 1
        }
    );

    // Reset work to a clean behind state and merge it
    git(&work, &["reset", "--hard", "HEAD~1"]);
    assert_eq!(check_repo_status(&work), RepoStatus::Behind(1));
    merge_ff_only(&work).unwrap();
    assert_eq!(check_repo_status(&work), RepoStatus::UpToDate);

    // Dirty alone, then dirty + behind: counts must survive the dirty flag
    std::fs::write(work.join("untracked.txt"), "wip").unwrap();
    assert_eq!(
        check_repo_status(&work),
        RepoStatus::Dirty {
            ahead: 0,
            behind: 0
        }
    );
    git(&other, &["commit", "--allow-empty", "-m", "four"]);
    git(&other, &["push", "origin", "main"]);
    assert_eq!(
        check_repo_status(&work),
        RepoStatus::Dirty {
            ahead: 0,
            behind: 1
        }
    );

    let _ = origin; // keep tmp alive for the assertions above
    let _ = other;
}
