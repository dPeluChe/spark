//! Repo manager tab state: managed repo list, clone input, clone summary.

use crate::scanner::repo_manager::ManagedRepo;
use std::collections::HashSet;
use std::path::PathBuf;

pub struct RepoManagerModel {
    pub repos: Vec<ManagedRepo>,
    pub cursor: usize,
    pub checked: HashSet<usize>,
    pub root: PathBuf,
    pub root_source: &'static str,
    pub clone_input: String,
    pub clone_error: Option<String>,
    pub cloning: bool,
    /// Last successful clone — drives the post-clone summary screen.
    pub last_clone: Option<CloneSummary>,
}

pub struct CloneSummary {
    pub repo_path: String,
    pub repo_name: String,
    pub remote_url: String,
    pub alias_cmd: String,
    pub short_path: String,
}

impl RepoManagerModel {
    pub fn new(repos_root: &std::path::Path, root_source: &'static str) -> Self {
        Self {
            repos: Vec::new(),
            cursor: 0,
            checked: HashSet::new(),
            root: repos_root.to_path_buf(),
            root_source,
            clone_input: String::new(),
            clone_error: None,
            cloning: false,
            last_clone: None,
        }
    }
}
