mod app;
mod config;
mod core;
mod updater;
mod scanner;
mod tui;
mod utils;

use std::io;
use std::path::PathBuf;
use clap::{Parser, Subcommand};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

/// SPARK — Developer Operations Platform
#[derive(Parser)]
#[command(name = "spark", version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Start in scanner mode only (skip updater)
    #[arg(long)]
    scan_only: bool,

    /// Start in updater mode only (skip scanner tab)
    #[arg(long)]
    update_only: bool,

    /// Show what would be done without making changes
    #[arg(long)]
    dry_run: bool,

    /// Override scan directories (comma-separated)
    #[arg(long, value_delimiter = ',')]
    scan_dir: Option<Vec<PathBuf>>,

    /// Override max scan depth
    #[arg(long)]
    max_depth: Option<usize>,
}

#[derive(Subcommand)]
enum Commands {
    /// Clone/sync a remote repository (ghq-compatible)
    #[command(alias = "get")]
    Clone {
        /// Git URL, or owner/repo shorthand (defaults to github.com)
        url: String,
        /// Clone via SSH instead of HTTPS
        #[arg(short = 'p')]
        ssh: bool,
        /// Do a shallow clone
        #[arg(long)]
        shallow: bool,
    },
    /// List local repositories
    List {
        /// Print full paths instead of relative
        #[arg(short = 'p', long = "full-path")]
        full_path: bool,
        /// Filter repos by query
        query: Option<String>,
    },
    /// Show repositories root path
    Root {
        /// Set a new repos root path
        #[arg(long)]
        set: Option<PathBuf>,
    },
    /// Remove a local repository
    Rm {
        /// Repository path, name, or owner/name
        query: String,
    },
    /// Show or update spark configuration
    Config {
        /// Show a specific config key
        key: Option<String>,
        /// Set a config key to a value
        #[arg(long)]
        set: Option<String>,
    },
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    utils::shell::init_log();
    let cli = Cli::parse();

    let mut config = config::SparkConfig::load();
    if let Some(dirs) = cli.scan_dir {
        config.scan_directories = dirs;
    }
    if let Some(depth) = cli.max_depth {
        config.max_scan_depth = depth;
    }

    // Handle subcommands (no TUI)
    if let Some(cmd) = cli.command {
        return handle_command(cmd, &mut config);
    }

    // TUI mode
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        crossterm::event::EnableFocusChange,
        crossterm::event::EnableMouseCapture,
    )?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = app::run(
        &mut terminal,
        config,
        cli.scan_only,
        cli.update_only,
        cli.dry_run,
    )
    .await;

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        crossterm::event::DisableFocusChange,
        crossterm::event::DisableMouseCapture,
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;

    if result.is_ok() {
        println!("\n  See you later, Space Cowboy...");
        println!("  Spark sequence complete.\n");
    }

    result
}

fn handle_command(cmd: Commands, config: &mut config::SparkConfig) -> color_eyre::Result<()> {
    match cmd {
        Commands::Clone { url, ssh, shallow } => {
            // Expand shorthand: "owner/repo" -> "https://github.com/owner/repo"
            let full_url = expand_url(&url, ssh);
            println!("  Cloning into {}/", config.repos_root.display());

            let clone_result = if shallow {
                scanner::repo_manager::clone_repo_shallow(&full_url, &config.repos_root)
            } else {
                scanner::repo_manager::clone_repo(&full_url, &config.repos_root)
            };

            match clone_result {
                Ok(path) => {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    let home = std::env::var("HOME").unwrap_or_default();
                    let display = if path.starts_with(&home) {
                        format!("~{}", &path.display().to_string()[home.len()..])
                    } else {
                        path.display().to_string()
                    };
                    println!("  ✔ {}\n", display);
                    println!("  cd {}", path.display());
                    println!("  alias {}='cd {}'", name.replace('-', "_"), display);
                }
                Err(e) => {
                    eprintln!("  ✘ {}", e);
                    std::process::exit(1);
                }
            }
        }

        Commands::List { full_path, query } => {
            let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
            if repos.is_empty() {
                println!("  No repos in {}", config.repos_root.display());
                println!("  Use: spark clone <url>");
                return Ok(());
            }

            let filtered: Vec<_> = match &query {
                Some(q) => {
                    let q = q.to_lowercase();
                    repos.iter().filter(|r| {
                        r.name.to_lowercase().contains(&q)
                            || r.owner.to_lowercase().contains(&q)
                            || r.host.to_lowercase().contains(&q)
                    }).collect()
                }
                None => repos.iter().collect(),
            };

            for repo in &filtered {
                if full_path {
                    println!("{}", repo.path.display());
                } else {
                    println!("{}/{}/{}", repo.host, repo.owner, repo.name);
                }
            }
        }

        Commands::Root { set } => {
            if let Some(new_root) = set {
                let path = if new_root.starts_with("~") {
                    let home = dirs::home_dir().unwrap_or_default();
                    home.join(new_root.strip_prefix("~/").unwrap_or(&new_root))
                } else {
                    std::fs::canonicalize(&new_root).unwrap_or(new_root)
                };

                if !path.exists() {
                    std::fs::create_dir_all(&path)?;
                    println!("  Created: {}", path.display());
                }

                config.repos_root = path.clone();
                config.save()?;
                println!("  repos_root set to: {}", path.display());
                println!("  Saved to ~/.config/spark/config.toml");
            } else {
                println!("{}", config.repos_root.display());
            }
        }

        Commands::Rm { query } => {
            let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
            let q = query.to_lowercase();
            let matches: Vec<_> = repos.iter().filter(|r| {
                r.name.to_lowercase() == q
                    || format!("{}/{}", r.owner, r.name).to_lowercase() == q
                    || format!("{}/{}/{}", r.host, r.owner, r.name).to_lowercase() == q
                    || r.path.display().to_string().to_lowercase().contains(&q)
            }).collect();

            if matches.is_empty() {
                eprintln!("  No repo matching '{}' found", query);
                std::process::exit(1);
            }
            if matches.len() > 1 {
                eprintln!("  Ambiguous: {} repos match '{}'", matches.len(), query);
                for m in &matches {
                    eprintln!("    {}/{}/{}", m.host, m.owner, m.name);
                }
                std::process::exit(1);
            }

            let repo = matches[0];
            println!("  Remove {}/{}/{}?", repo.host, repo.owner, repo.name);
            println!("  Path: {}", repo.path.display());
            print!("  Confirm (y/N): ");
            io::Write::flush(&mut io::stdout())?;

            let mut answer = String::new();
            io::stdin().read_line(&mut answer)?;
            if answer.trim().to_lowercase() == "y" {
                if config.use_trash {
                    let trash_dir = dirs::data_dir()
                        .unwrap_or_else(|| PathBuf::from("/tmp"))
                        .join("spark-trash");
                    std::fs::create_dir_all(&trash_dir)?;
                    let dest = trash_dir.join(repo.name.clone());
                    std::fs::rename(&repo.path, &dest)?;
                    println!("  Moved to trash: {}", dest.display());
                } else {
                    std::fs::remove_dir_all(&repo.path)?;
                    println!("  Removed: {}", repo.path.display());
                }
            } else {
                println!("  Cancelled");
            }
        }

        Commands::Config { key, set } => {
            if let Some(value) = set {
                // Expect key to be provided when setting
                let key = key.unwrap_or_default();
                match key.as_str() {
                    "repos_root" => {
                        config.repos_root = PathBuf::from(&value);
                        config.save()?;
                        println!("  repos_root = {}", value);
                    }
                    "stale_threshold_days" => {
                        config.stale_threshold_days = value.parse().map_err(|_| {
                            color_eyre::eyre::eyre!("Invalid number: {}", value)
                        })?;
                        config.save()?;
                        println!("  stale_threshold_days = {}", value);
                    }
                    "max_scan_depth" => {
                        config.max_scan_depth = value.parse().map_err(|_| {
                            color_eyre::eyre::eyre!("Invalid number: {}", value)
                        })?;
                        config.save()?;
                        println!("  max_scan_depth = {}", value);
                    }
                    "use_trash" => {
                        config.use_trash = value.parse().map_err(|_| {
                            color_eyre::eyre::eyre!("Expected true/false: {}", value)
                        })?;
                        config.save()?;
                        println!("  use_trash = {}", value);
                    }
                    _ => {
                        eprintln!("  Unknown key: {}", key);
                        eprintln!("  Available: repos_root, stale_threshold_days, max_scan_depth, use_trash");
                        std::process::exit(1);
                    }
                }
                println!("  Saved to ~/.config/spark/config.toml");
            } else {
                // Show config
                println!("  repos_root          = {}", config.repos_root.display());
                println!("  stale_threshold_days = {}", config.stale_threshold_days);
                println!("  large_artifact_threshold = {} bytes", config.large_artifact_threshold);
                println!("  use_trash           = {}", config.use_trash);
                println!("  max_scan_depth      = {}", config.max_scan_depth);
                println!("  scan_directories:");
                for dir in &config.scan_directories {
                    println!("    - {}", dir.display());
                }
                println!("\n  Config file: ~/.config/spark/config.toml");
                if let Some(key) = key {
                    println!("\n  Tip: spark config {} --set <value>", key);
                }
            }
        }
    }
    Ok(())
}

/// Expand shorthand URLs:
/// "owner/repo" -> "https://github.com/owner/repo"
/// Already full URL -> pass through
fn expand_url(input: &str, use_ssh: bool) -> String {
    // Already a full URL
    if input.starts_with("https://") || input.starts_with("http://") || input.starts_with("git@") {
        return input.to_string();
    }

    // owner/repo shorthand
    let parts: Vec<&str> = input.split('/').collect();
    if parts.len() == 2 {
        if use_ssh {
            return format!("git@github.com:{}/{}.git", parts[0], parts[1]);
        }
        return format!("https://github.com/{}/{}", parts[0], parts[1]);
    }

    // host/owner/repo
    if parts.len() == 3 {
        if use_ssh {
            return format!("git@{}:{}/{}.git", parts[0], parts[1], parts[2]);
        }
        return format!("https://{}/{}/{}", parts[0], parts[1], parts[2]);
    }

    input.to_string()
}
