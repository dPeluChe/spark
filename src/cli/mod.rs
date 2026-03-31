//! CLI command definitions and dispatch for spark subcommands.

mod repos;
mod system;
mod audit;

use std::path::PathBuf;
use clap::{Parser, Subcommand};
use crate::config;

/// SPARK — Developer Operations Platform
#[derive(Parser)]
#[command(name = "spark", version, about, long_about = None, term_width = 80)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Start in scanner mode only (skip updater)
    #[arg(long)]
    pub scan_only: bool,

    /// Start in updater mode only (skip scanner tab)
    #[arg(long)]
    pub update_only: bool,

    /// Show what would be done without making changes
    #[arg(long)]
    pub dry_run: bool,

    /// Override scan directories (comma-separated)
    #[arg(long, value_delimiter = ',')]
    pub scan_dir: Option<Vec<PathBuf>>,

    /// Override max scan depth
    #[arg(long)]
    pub max_depth: Option<usize>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Clone/sync a remote repository (ghq-compatible)
    #[command(alias = "get")]
    Clone {
        url: String,
        #[arg(short = 'p')]
        ssh: bool,
        #[arg(long)]
        shallow: bool,
    },
    /// List local repositories
    List {
        #[arg(short = 'p', long = "full-path")]
        full_path: bool,
        query: Option<String>,
    },
    /// Show repositories root path
    Root {
        #[arg(long)]
        set: Option<PathBuf>,
    },
    /// Remove a local repository
    Rm { query: String },
    /// Search repos and print matching paths (for AI agents and scripts)
    Search {
        query: String,
        #[arg(short = '1', long = "first")]
        first: bool,
    },
    /// Print path to a repo (use with: cd "$(spark cd <name>)")
    Cd { query: String },
    /// Initialize spark: setup shell integration and config
    Init,
    /// Show agent integration tips
    Agent,
    /// Generate shell completions (zsh, bash, fish)
    Completions { shell: clap_complete::Shell },
    /// Show or update spark configuration
    Config {
        key: Option<String>,
        #[arg(long)]
        set: Option<String>,
    },
    /// Check which repos need updating (fetch + compare with remote)
    Status { query: Option<String> },
    /// Pull repos that are behind remote (fast-forward only)
    Pull { query: String },
    /// Scan for exposed secrets and credentials (nothing leaves your machine)
    Audit {
        /// Directory to scan (defaults to current directory)
        path: Option<PathBuf>,
        /// Save report to a file
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
        /// Create a .sparkauditignore file with defaults
        #[arg(long = "init")]
        init_ignore: bool,
    },
    /// Validate installation and environment health
    Doctor,
}

pub fn handle_command(cmd: Commands, config: &mut config::SparkConfig) -> color_eyre::Result<()> {
    match cmd {
        Commands::Clone { url, ssh, shallow } => repos::cmd_clone(&url, ssh, shallow, config),
        Commands::List { full_path, query } => repos::cmd_list(full_path, query, config),
        Commands::Root { set } => system::cmd_root(set, config),
        Commands::Rm { query } => repos::cmd_rm(&query, config),
        Commands::Search { query, first } => repos::cmd_search(&query, first, config),
        Commands::Cd { query } => repos::cmd_cd(&query, config),
        Commands::Init => system::cmd_init(config),
        Commands::Agent => { system::cmd_agent(config); Ok(()) }
        Commands::Completions { shell } => { system::cmd_completions(shell); Ok(()) }
        Commands::Config { key, set } => system::cmd_config(key, set, config),
        Commands::Status { query } => { repos::cmd_status(query, config); Ok(()) }
        Commands::Pull { query } => { repos::cmd_pull(&query, config); Ok(()) }
        Commands::Audit { path, output, init_ignore } => { audit::cmd_audit(path, output, init_ignore); Ok(()) }
        Commands::Doctor => { system::cmd_doctor(config); Ok(()) }
    }
}

// ─── Shared helpers ───

pub(crate) fn filter_repo(r: &crate::scanner::repo_manager::ManagedRepo, q: &str) -> bool {
    r.name.to_lowercase().contains(q)
        || r.owner.to_lowercase().contains(q)
        || r.host.to_lowercase().contains(q)
        || format!("{}/{}", r.owner, r.name).to_lowercase().contains(q)
}

pub(crate) fn shorten_path(path: &str) -> String {
    crate::utils::fs::shorten_path(path)
}

pub(crate) fn expand_url(input: &str, use_ssh: bool) -> String {
    if input.starts_with("https://") || input.starts_with("http://") || input.starts_with("git@") {
        return input.to_string();
    }
    let parts: Vec<&str> = input.split('/').collect();
    if parts.len() == 2 {
        return if use_ssh { format!("git@github.com:{}/{}.git", parts[0], parts[1]) }
            else { format!("https://github.com/{}/{}", parts[0], parts[1]) };
    }
    if parts.len() == 3 {
        return if use_ssh { format!("git@{}:{}/{}.git", parts[0], parts[1], parts[2]) }
            else { format!("https://{}/{}/{}", parts[0], parts[1], parts[2]) };
    }
    input.to_string()
}
