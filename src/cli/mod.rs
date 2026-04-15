//! CLI command definitions and dispatch for spark subcommands.

mod repos;
mod system;
mod audit;
mod certs;
mod tags;
mod ingest;
mod ports;

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
    Status {
        query: Option<String>,
        /// Filter by tag
        #[arg(long = "tag", short = 't')]
        tag: Option<String>,
    },
    /// Pull repos that are behind remote (fast-forward only)
    Pull {
        query: String,
        /// Filter by tag (pull all repos with this tag)
        #[arg(long = "tag", short = 't')]
        tag: Option<String>,
    },
    /// Scan for exposed secrets and credentials
    Audit {
        /// Directory to scan (defaults to current directory)
        path: Option<PathBuf>,
        /// Save report to a file
        #[arg(short = 'o', long = "output")]
        output: Option<PathBuf>,
        /// Create a .sparkauditignore file with defaults
        #[arg(long = "init")]
        init_ignore: bool,
        /// Skip dependency vulnerability check (no network)
        #[arg(long = "offline")]
        offline: bool,
        /// Only run dependency vulnerability check
        #[arg(long = "deps")]
        deps_only: bool,
    },
    /// Scan SSL/TLS certificates (files + macOS Keychain)
    Certs {
        /// Directory to scan for cert files (defaults to current directory)
        path: Option<PathBuf>,
        /// Only scan macOS Keychain (skip file scan)
        #[arg(long = "keychain")]
        keychain_only: bool,
        /// (default shows all — this flag is kept for compatibility)
        #[arg(long = "all", hide = true)]
        show_all: bool,
        /// Show only expired certificates
        #[arg(long = "expired")]
        expired_only: bool,
        /// Show summary table only (no details)
        #[arg(long = "summary")]
        summary_only: bool,
    },
    /// Generate LLM-ready context file for a repo (uses repomix)
    Ingest {
        /// Repo to ingest (name or owner/name). Omit to list existing.
        query: Option<String>,
        /// Ingest ALL repos (slow — runs repomix on each)
        #[arg(long = "all")]
        all: bool,
        /// Compress with Tree-sitter (reduces ~70% tokens)
        #[arg(long = "compress")]
        compress: bool,
        /// Print the ingest content to stdout (for piping to LLMs)
        #[arg(long = "read")]
        read: bool,
    },
    /// Manage repository tags/groups
    #[command(alias = "tags")]
    Tag {
        #[command(subcommand)]
        action: TagAction,
    },
    /// Inspect processes and ports (replaces lsof + ps aux | grep)
    #[command(alias = "ports")]
    Ps {
        /// Search processes by name (shows their ports if listening)
        query: Option<String>,
        /// Show all ports including system processes (default: dev servers only)
        #[arg(long = "all", short = 'a')]
        all: bool,
        /// Kill by port, PID, or name. Use with query for non-interactive: spark ps node --kill
        #[arg(long = "kill", short = 'k', num_args = 0..=1, default_missing_value = "")]
        kill: Option<String>,
    },
    /// Validate installation and environment health
    Doctor,
}

#[derive(Subcommand)]
pub enum TagAction {
    /// Add a tag to a repository
    Add {
        /// Repository (name or owner/name)
        repo: String,
        /// Tag name
        tag: String,
    },
    /// Remove a tag from a repository
    Remove {
        /// Repository (name or owner/name)
        repo: String,
        /// Tag name
        tag: String,
    },
    /// List all tags or repos in a tag
    List {
        /// Tag name (omit to see all tags)
        tag: Option<String>,
    },
    /// Delete an entire tag
    Delete {
        /// Tag to delete
        tag: String,
    },
    /// Rename a tag
    Rename {
        /// Current tag name
        old: String,
        /// New tag name
        new_name: String,
    },
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
        Commands::Status { query, tag } => { repos::cmd_status(query, tag, config); Ok(()) }
        Commands::Pull { query, tag } => { repos::cmd_pull(&query, tag, config); Ok(()) }
        Commands::Audit { path, output, init_ignore, offline, deps_only } => {
            if deps_only { audit::cmd_audit_deps(path); }
            else { audit::cmd_audit(path, output, init_ignore, offline); }
            Ok(())
        }
        Commands::Ingest { query, all, compress, read } => { ingest::cmd_ingest(query, all, compress, read, config); Ok(()) }
        Commands::Tag { action } => { tags::cmd_tag(action, config); Ok(()) }
        Commands::Certs { path, keychain_only, show_all: _, expired_only, summary_only } => {
            certs::cmd_certs(path, keychain_only, expired_only, summary_only); Ok(())
        }
        Commands::Ps { all, query, kill } => { ports::cmd_ports(all, query, kill); Ok(()) }
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
    crate::scanner::common::shorten_path(path)
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
