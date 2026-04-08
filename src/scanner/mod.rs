//! Repository scanner: discovery, health analysis, and artifact cleanup.

pub mod common;
pub mod repo_scanner;
pub mod space_analyzer;
pub mod health;
pub mod cleaner;
pub mod port_scanner;
pub mod repo_manager;
pub mod system_cleaner;
mod system_categories;
pub mod secret_scanner;
pub mod history_scanner;
pub mod code_patterns;
pub mod dep_scanner;
pub mod cert_scanner;
pub mod repo_tags;
pub mod repo_ingest;
