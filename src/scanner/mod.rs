//! Repository scanner: discovery, health analysis, and artifact cleanup.

pub mod cert_scanner;
pub mod cleaner;
pub mod code_patterns;
pub mod common;
pub mod dep_scanner;
pub mod health;
pub mod history_scanner;
pub mod port_scanner;
pub mod repo_ingest;
pub mod repo_manager;
pub mod repo_scanner;
pub mod repo_tags;
pub mod secret_scanner;
pub mod space_analyzer;
mod system_categories;
pub mod system_cleaner;
