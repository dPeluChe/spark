//! Repository tagging system for organizing managed repos into groups.
//!
//! Tags are stored in ~/.config/spark/repo_tags.json
//! A repo can belong to multiple tags. Tags can be used to filter
//! status, pull, and list operations.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Tag storage: tag_name -> set of repo keys (host/owner/name)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct RepoTags {
    pub tags: BTreeMap<String, BTreeSet<String>>,
}

#[allow(dead_code)]
impl RepoTags {
    pub fn new() -> Self {
        Self { tags: BTreeMap::new() }
    }

    /// Add a repo to a tag (creates tag if it doesn't exist)
    pub fn add(&mut self, repo_key: &str, tag: &str) {
        self.tags.entry(tag.to_lowercase()).or_default().insert(repo_key.to_string());
    }

    /// Remove a repo from a tag
    pub fn remove(&mut self, repo_key: &str, tag: &str) -> bool {
        if let Some(repos) = self.tags.get_mut(&tag.to_lowercase()) {
            let removed = repos.remove(repo_key);
            if repos.is_empty() {
                self.tags.remove(&tag.to_lowercase());
            }
            removed
        } else {
            false
        }
    }

    /// Get all tags for a repo
    pub fn tags_for_repo(&self, repo_key: &str) -> Vec<String> {
        self.tags.iter()
            .filter(|(_, repos)| repos.contains(repo_key))
            .map(|(tag, _)| tag.clone())
            .collect()
    }

    /// Get all repos for a tag
    pub fn repos_for_tag(&self, tag: &str) -> Vec<String> {
        self.tags.get(&tag.to_lowercase())
            .map(|repos| repos.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get all tag names
    pub fn all_tags(&self) -> Vec<String> {
        self.tags.keys().cloned().collect()
    }

    /// Check if a repo has a specific tag
    pub fn has_tag(&self, repo_key: &str, tag: &str) -> bool {
        self.tags.get(&tag.to_lowercase())
            .map(|repos| repos.contains(repo_key))
            .unwrap_or(false)
    }

    /// Delete an entire tag
    pub fn delete_tag(&self, _tag: &str) -> Self {
        let mut new = self.clone();
        new.tags.remove(&_tag.to_lowercase());
        new
    }

    /// Rename a tag
    pub fn rename_tag(&mut self, old: &str, new_name: &str) -> bool {
        if let Some(repos) = self.tags.remove(&old.to_lowercase()) {
            self.tags.insert(new_name.to_lowercase(), repos);
            true
        } else {
            false
        }
    }
}

/// Build a repo key from host/owner/name
pub fn repo_key(host: &str, owner: &str, name: &str) -> String {
    format!("{}/{}/{}", host, owner, name)
}

/// Build a repo key from just owner/name (assumes github.com)
#[allow(dead_code)]
pub fn repo_key_short(owner: &str, name: &str) -> String {
    format!("github.com/{}/{}", owner, name)
}

// --- Persistence ---

fn tags_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("spark")
        .join("repo_tags.json")
}

/// Load tags from disk
pub fn load_tags() -> RepoTags {
    let path = tags_path();
    if !path.exists() {
        return RepoTags::new();
    }
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| RepoTags::new()),
        Err(_) => RepoTags::new(),
    }
}

/// Save tags to disk
pub fn save_tags(tags: &RepoTags) {
    let path = tags_path();
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new("/tmp")));
    let _ = std::fs::write(path, serde_json::to_string_pretty(tags).unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get() {
        let mut tags = RepoTags::new();
        tags.add("github.com/user/repo1", "learning");
        tags.add("github.com/user/repo2", "learning");
        tags.add("github.com/user/repo1", "ai-tools");

        assert_eq!(tags.repos_for_tag("learning").len(), 2);
        assert_eq!(tags.tags_for_repo("github.com/user/repo1").len(), 2);
    }

    #[test]
    fn test_remove() {
        let mut tags = RepoTags::new();
        tags.add("github.com/user/repo1", "learning");
        tags.add("github.com/user/repo2", "learning");

        assert!(tags.remove("github.com/user/repo1", "learning"));
        assert_eq!(tags.repos_for_tag("learning").len(), 1);
    }

    #[test]
    fn test_remove_last_deletes_tag() {
        let mut tags = RepoTags::new();
        tags.add("github.com/user/repo1", "temp");
        tags.remove("github.com/user/repo1", "temp");
        assert!(tags.all_tags().is_empty());
    }

    #[test]
    fn test_case_insensitive() {
        let mut tags = RepoTags::new();
        tags.add("github.com/user/repo1", "Learning");
        assert!(tags.has_tag("github.com/user/repo1", "learning"));
        assert!(tags.has_tag("github.com/user/repo1", "LEARNING"));
    }

    #[test]
    fn test_rename() {
        let mut tags = RepoTags::new();
        tags.add("github.com/user/repo1", "old-name");
        tags.rename_tag("old-name", "new-name");
        assert!(tags.repos_for_tag("old-name").is_empty());
        assert_eq!(tags.repos_for_tag("new-name").len(), 1);
    }

    #[test]
    fn test_repo_key() {
        assert_eq!(repo_key("github.com", "user", "repo"), "github.com/user/repo");
        assert_eq!(repo_key_short("user", "repo"), "github.com/user/repo");
    }
}
