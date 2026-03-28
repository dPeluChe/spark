mod app;
mod config;
mod core;
mod updater;
mod scanner;
mod tui;
mod utils;

use std::io;
use std::path::PathBuf;
use clap::{CommandFactory, Parser, Subcommand};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

/// SPARK — Developer Operations Platform
#[derive(Parser)]
#[command(name = "spark", version, about, long_about = None, term_width = 80)]
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
    /// Search repos and print matching paths (for AI agents and scripts)
    Search {
        /// Search query (matches repo name, owner, or host)
        query: String,
        /// Print only the first match
        #[arg(short = '1', long = "first")]
        first: bool,
    },
    /// Print path to a repo (use with: cd "$(spark cd <name>)")
    Cd {
        /// Repo name to find
        query: String,
    },
    /// Initialize spark: setup shell integration and config
    Init,
    /// Show agent integration tips
    Agent,
    /// Generate shell completions (zsh, bash, fish)
    Completions {
        /// Shell type
        shell: clap_complete::Shell,
    },
    /// Show or update spark configuration
    Config {
        /// Show a specific config key
        key: Option<String>,
        /// Set a config key to a value
        #[arg(long)]
        set: Option<String>,
    },
    /// Check which repos need updating (fetch + compare with remote)
    Status {
        /// Filter repos by query
        query: Option<String>,
    },
    /// Pull repos that are behind remote (fast-forward only)
    Pull {
        /// Repo name/owner to pull, or "all" to pull every repo behind
        query: String,
    },
    /// Validate installation and environment health
    Doctor,
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
                            || format!("{}/{}", r.owner, r.name).to_lowercase().contains(&q)
                    }).collect()
                }
                None => repos.iter().collect(),
            };

            if full_path {
                for repo in &filtered {
                    println!("{}", repo.path.display());
                }
            } else {
                print_repos_tree(&filtered);
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

        Commands::Search { query, first } => {
            let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
            let q = query.to_lowercase();
            let matches: Vec<_> = repos.iter().filter(|r| {
                r.name.to_lowercase().contains(&q)
                    || r.owner.to_lowercase().contains(&q)
                    || r.host.to_lowercase().contains(&q)
            }).collect();

            if matches.is_empty() {
                eprintln!("  No repos matching '{}'", query);
                std::process::exit(1);
            }

            if first {
                println!("{}", matches[0].path.display());
            } else {
                let home = std::env::var("HOME").unwrap_or_default();
                let cache = scanner::repo_manager::load_status_cache();
                for (i, repo) in matches.iter().enumerate() {
                    if i > 0 { println!(); }
                    let path = repo.path.display().to_string();
                    let short = if path.starts_with(&home) {
                        format!("~{}", &path[home.len()..])
                    } else {
                        path
                    };
                    let age = repo.last_commit.as_deref().unwrap_or("-");
                    let key = repo.path.display().to_string();
                    let status = cache.get(&key)
                        .filter(|(_, ts)| scanner::repo_manager::is_cache_valid(*ts))
                        .map(|(s, _)| scanner::repo_manager::string_to_status(s));
                    let status_str = status.as_ref()
                        .map(|s| format!("{}", s))
                        .unwrap_or_else(|| "unknown (run spark status)".into());

                    println!("  repo:   {}", repo.name);
                    println!("  owner:  {}", repo.owner);
                    println!("  host:   {}", repo.host);
                    println!("  branch: {}", repo.branch);
                    println!("  status: {}", status_str);
                    println!("  commit: {}", age);
                    println!("  path:   {}", short);
                }
            }
        }

        Commands::Init => {
            println!("  Spark Init — Setting up your environment\n");

            // 1. Detect shell
            let shell = std::env::var("SHELL").unwrap_or_default();
            let rc_file = if shell.contains("zsh") {
                dirs::home_dir().unwrap_or_default().join(".zshrc")
            } else if shell.contains("bash") {
                dirs::home_dir().unwrap_or_default().join(".bashrc")
            } else {
                dirs::home_dir().unwrap_or_default().join(".profile")
            };
            println!("  Shell: {}", shell);
            println!("  Config: {}\n", rc_file.display());

            // 2. Check what's already configured
            let rc_content = std::fs::read_to_string(&rc_file).unwrap_or_default();
            let has_spark_cd = rc_content.contains("spark-cd");
            let _has_spark_path = std::env::var("PATH").unwrap_or_default()
                .contains("spark");

            // 3. Build the block to add
            let spark_block = r#"
# --- Spark DevOps Platform ---
# Navigate to managed repos
spark-cd() { cd "$(spark cd "$1")" ; }
# --- End Spark ---"#;

            if has_spark_cd {
                println!("  [✓] spark-cd already configured");
            } else {
                print!("  Add spark-cd to {}? (y/N): ", rc_file.display());
                io::Write::flush(&mut io::stdout())?;
                let mut answer = String::new();
                io::stdin().read_line(&mut answer)?;

                if answer.trim().to_lowercase() == "y" {
                    use std::io::Write;
                    let mut f = std::fs::OpenOptions::new()
                        .append(true)
                        .open(&rc_file)?;
                    writeln!(f, "{}", spark_block)?;
                    println!("  [✓] Added spark-cd to {}", rc_file.display());
                } else {
                    println!("  [—] Skipped");
                }
            }

            // 4. Create config dir and default whitelist
            let config_dir = dirs::config_dir().unwrap_or_default().join("spark");
            std::fs::create_dir_all(&config_dir)?;

            let whitelist_path = config_dir.join("whitelist.txt");
            if !whitelist_path.exists() {
                std::fs::write(&whitelist_path,
                    "# Spark whitelist — paths listed here will be skipped during system cleanup\n\
                     # One path per line. Use ~ for home directory.\n\
                     # Example:\n\
                     # ~/Library/Caches/important-app\n\
                     # ~/.cargo/registry\n"
                )?;
                println!("  [✓] Created {}", whitelist_path.display());
            } else {
                println!("  [✓] Whitelist already exists");
            }

            // 5. Install shell completions
            let home = dirs::home_dir().unwrap_or_default();
            let shell_name = shell.rsplit('/').next().unwrap_or("");
            match shell_name {
                "zsh" => {
                    let comp_dir = home.join(".zsh/completions");
                    let _ = std::fs::create_dir_all(&comp_dir);
                    let comp_file = comp_dir.join("_spark");
                    if !comp_file.exists() {
                        let mut buf = Vec::new();
                        clap_complete::generate(
                            clap_complete::Shell::Zsh,
                            &mut Cli::command(),
                            "spark",
                            &mut buf,
                        );
                        let _ = std::fs::write(&comp_file, buf);
                        println!("  [✓] Installed zsh completions");
                    } else {
                        println!("  [✓] Zsh completions already installed");
                    }
                }
                "bash" => {
                    let comp_dir = home.join(".local/share/bash-completion/completions");
                    let _ = std::fs::create_dir_all(&comp_dir);
                    let comp_file = comp_dir.join("spark");
                    if !comp_file.exists() {
                        let mut buf = Vec::new();
                        clap_complete::generate(
                            clap_complete::Shell::Bash,
                            &mut Cli::command(),
                            "spark",
                            &mut buf,
                        );
                        let _ = std::fs::write(&comp_file, buf);
                        println!("  [✓] Installed bash completions");
                    } else {
                        println!("  [✓] Bash completions already installed");
                    }
                }
                _ => {}
            }

            // 6. Show summary
            println!("\n  Setup complete!\n");
            println!("  Repos root:  {}", config.repos_root.display());
            println!("  Config:      {}", config_dir.display());
            println!("  Whitelist:   {}", whitelist_path.display());
            println!("\n  Run `source {}` to activate, then:", rc_file.display());
            println!("    spark-cd <repo>    Navigate to a repo");
            println!("    spark              Open the TUI");
            println!("    spark clone <url>  Clone a repo");
            println!("    spark agent        Integration tips for AI agents");
        }

        Commands::Cd { query } => {
            let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
            let q = query.to_lowercase();
            // Exact name match first, then partial
            let exact = repos.iter().find(|r| r.name.to_lowercase() == q);
            let found = exact.or_else(|| repos.iter().find(|r|
                r.name.to_lowercase().contains(&q)
            ));

            match found {
                Some(repo) => println!("{}", repo.path.display()),
                None => {
                    eprintln!("  No repo matching '{}'", query);
                    std::process::exit(1);
                }
            }
        }

        Commands::Agent => {
            let root = config.repos_root.display();
            println!("  Spark Repo Manager — Agent Integration\n");
            println!("  Your repos live at: {}\n", root);
            println!("  For AI agents (Claude Code, Cursor, Codex):\n");
            println!("  1. Tell your agent:");
            println!("     \"Repos managed by spark. Run `spark cd <name>` to find paths.\"\n");
            println!("  2. Add to CLAUDE.md or .cursorrules:");
            println!("     Repos root: {}", root);
            println!("     Find repo: spark cd <name>\n");
            println!("  3. Shell function (add to ~/.zshrc):");
            println!("     spark-cd() {{ cd \"$(spark cd \"$1\")\" ; }}\n");
            println!("  4. Commands:");
            println!("     spark cd zed           # Print path to zed repo");
            println!("     spark search zed       # Search all matching repos");
            println!("     spark list -p           # All repo paths");
            println!("     spark clone user/repo  # Clone to managed root");
            println!("     spark root             # Show repos root path");
        }

        Commands::Completions { shell } => {
            clap_complete::generate(
                shell,
                &mut Cli::command(),
                "spark",
                &mut io::stdout(),
            );
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
        Commands::Status { query } => {
            let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
            let filtered: Vec<_> = match &query {
                Some(q) => {
                    let q = q.to_lowercase();
                    repos.iter().filter(|r| {
                        r.name.to_lowercase().contains(&q)
                            || r.owner.to_lowercase().contains(&q)
                            || r.host.to_lowercase().contains(&q)
                            || format!("{}/{}", r.owner, r.name).to_lowercase().contains(&q)
                    }).collect()
                }
                None => repos.iter().collect(),
            };

            if filtered.is_empty() {
                println!("  No repos found");
                return Ok(());
            }

            println!("  Checking {} repos...\n", filtered.len());

            let cache = scanner::repo_manager::load_status_cache();
            let mut statuses: Vec<(&scanner::repo_manager::ManagedRepo, scanner::repo_manager::RepoStatus)> = Vec::new();

            for (i, repo) in filtered.iter().enumerate() {
                eprint!("\r  [{}/{}] {}/{}", i + 1, filtered.len(), repo.owner, repo.name);
                let key = repo.path.display().to_string();
                let status = if let Some((cached, ts)) = cache.get(&key) {
                    if scanner::repo_manager::is_cache_valid(*ts) {
                        scanner::repo_manager::string_to_status(cached)
                    } else {
                        let s = scanner::repo_manager::check_repo_status(&repo.path);
                        scanner::repo_manager::save_status_to_cache(&key, &scanner::repo_manager::status_to_string(&s));
                        s
                    }
                } else {
                    let s = scanner::repo_manager::check_repo_status(&repo.path);
                    scanner::repo_manager::save_status_to_cache(&key, &scanner::repo_manager::status_to_string(&s));
                    s
                };
                statuses.push((repo, status));
            }
            eprintln!("\r{}\r", " ".repeat(60));

            print_status_table(&statuses);

            let behind = statuses.iter().filter(|(_, s)| matches!(s, scanner::repo_manager::RepoStatus::Behind(_))).count();
            let diverged = statuses.iter().filter(|(_, s)| matches!(s, scanner::repo_manager::RepoStatus::Diverged { .. })).count();
            if behind > 0 || diverged > 0 {
                println!("\n  {} repos need pull", behind + diverged);
                println!("  spark pull <name>   pull a specific repo");
                println!("  spark pull all      pull all behind repos");
            }
        }

        Commands::Pull { query } => {
            let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
            let is_all = query.to_lowercase() == "all";
            let filtered: Vec<_> = if is_all {
                repos.iter().collect()
            } else {
                let q = query.to_lowercase();
                // Try exact owner/name or exact name first
                let exact: Vec<_> = repos.iter().filter(|r| {
                    format!("{}/{}", r.owner, r.name).to_lowercase() == q
                        || r.name.to_lowercase() == q
                }).collect();
                if !exact.is_empty() {
                    exact
                } else {
                    repos.iter().filter(|r| {
                        r.name.to_lowercase().contains(&q)
                            || r.owner.to_lowercase().contains(&q)
                            || r.host.to_lowercase().contains(&q)
                            || format!("{}/{}", r.owner, r.name).to_lowercase().contains(&q)
                    }).collect()
                }
            };

            if filtered.is_empty() {
                eprintln!("  No repos matching '{}'", query);
                std::process::exit(1);
            }

            if !is_all && filtered.len() > 1 {
                eprintln!("  {} repos match '{}'. Be more specific:\n", filtered.len(), query);
                for r in &filtered {
                    eprintln!("    spark pull {}/{}", r.owner, r.name);
                }
                std::process::exit(1);
            }

            println!("  Checking {} repos for updates...\n", filtered.len());

            let mut pulled = 0usize;
            let mut skipped = 0usize;
            let mut errors = Vec::new();

            for (i, repo) in filtered.iter().enumerate() {
                eprint!("\r  [{}/{}] {}/{}", i + 1, filtered.len(), repo.owner, repo.name);
                let status = scanner::repo_manager::check_repo_status(&repo.path);
                let key = repo.path.display().to_string();

                match &status {
                    scanner::repo_manager::RepoStatus::Behind(_)
                    | scanner::repo_manager::RepoStatus::Diverged { .. } => {
                        match scanner::repo_manager::pull_repo(&repo.path) {
                            Ok(_) => {
                                pulled += 1;
                                scanner::repo_manager::save_status_to_cache(&key, "up_to_date");
                            }
                            Err(e) => {
                                errors.push(format!("{}/{}: {}", repo.owner, repo.name, e));
                                scanner::repo_manager::save_status_to_cache(&key, &scanner::repo_manager::status_to_string(&status));
                            }
                        }
                    }
                    _ => {
                        skipped += 1;
                        scanner::repo_manager::save_status_to_cache(&key, &scanner::repo_manager::status_to_string(&status));
                    }
                }
            }
            eprintln!("\r{}\r", " ".repeat(60));

            if pulled > 0 {
                println!("  {} repos pulled", pulled);
            }
            if skipped > 0 {
                println!("  {} repos already up to date", skipped);
            }
            for e in &errors {
                eprintln!("  Error: {}", e);
            }
            if pulled == 0 && errors.is_empty() {
                println!("  All repos up to date");
            }
        }

        Commands::Doctor => {
            run_doctor(config);
        }
    }
    Ok(())
}

fn run_doctor(config: &config::SparkConfig) {
    println!("\n  SPARK Doctor — Environment Health Check\n");

    let mut pass = 0u32;
    let mut fail = 0u32;
    let mut warn = 0u32;

    // Helper closures
    let mut check = |label: &str, ok: bool, fix: &str| {
        if ok {
            println!("  \x1b[32m✓\x1b[0m {}", label);
            pass += 1;
        } else {
            println!("  \x1b[31m✗\x1b[0m {}  \x1b[90m→ {}\x1b[0m", label, fix);
            fail += 1;
        }
    };

    // 1. Binary
    let version = env!("CARGO_PKG_VERSION");
    check("spark binary", true, "");
    println!("    version: {}", version);

    let binary_path = std::env::current_exe().unwrap_or_default();
    println!("    path:    {}", binary_path.display());

    // 2. Git
    let git_ok = std::process::Command::new("git").arg("--version").output()
        .map(|o| o.status.success()).unwrap_or(false);
    check("git available", git_ok, "install git: brew install git");

    // 3. Repos root
    let root_exists = config.repos_root.exists();
    let root_writable = root_exists && std::fs::metadata(&config.repos_root)
        .map(|m| !m.permissions().readonly()).unwrap_or(false);
    check(
        &format!("repos root exists ({})", config.repos_root.display()),
        root_exists,
        &format!("mkdir -p {}", config.repos_root.display()),
    );
    if root_exists {
        check("repos root writable", root_writable, "check permissions");
        let repos = scanner::repo_manager::list_managed_repos(&config.repos_root);
        println!("    repos:   {}", repos.len());
    }

    // 4. Config directory
    let config_dir = dirs::config_dir().unwrap_or_default().join("spark");
    let config_file = config_dir.join("config.toml");
    check(
        "config directory",
        config_dir.exists(),
        &format!("spark init  (creates {})", config_dir.display()),
    );
    check(
        "config.toml",
        config_file.exists(),
        "spark init  (creates default config)",
    );

    // 5. Whitelist
    let whitelist = config_dir.join("whitelist.txt");
    check(
        "whitelist.txt",
        whitelist.exists(),
        "spark init  (creates whitelist)",
    );

    // 6. Shell integration
    let shell = std::env::var("SHELL").unwrap_or_default();
    let rc_file = if shell.contains("zsh") {
        dirs::home_dir().unwrap_or_default().join(".zshrc")
    } else if shell.contains("bash") {
        dirs::home_dir().unwrap_or_default().join(".bashrc")
    } else {
        dirs::home_dir().unwrap_or_default().join(".profile")
    };
    let rc_content = std::fs::read_to_string(&rc_file).unwrap_or_default();
    let has_spark_cd = rc_content.contains("spark-cd");
    check(
        &format!("spark-cd in {}", rc_file.file_name().unwrap_or_default().to_string_lossy()),
        has_spark_cd,
        "spark init  (adds spark-cd function)",
    );

    // 7. Shell completions
    let home = dirs::home_dir().unwrap_or_default();
    let has_completions = if shell.contains("zsh") {
        home.join(".zsh/completions/_spark").exists()
    } else if shell.contains("bash") {
        home.join(".local/share/bash-completion/completions/spark").exists()
    } else {
        false
    };
    check(
        "shell completions",
        has_completions,
        "spark init  (installs completions)",
    );

    // 8. Platform-specific tools
    if cfg!(target_os = "macos") {
        let lsof_ok = std::process::Command::new("lsof").arg("-v").output()
            .map(|_| true).unwrap_or(false);
        check("lsof (port scanner)", lsof_ok, "should be pre-installed on macOS");

        // Docker (optional)
        let docker_ok = std::process::Command::new("docker").arg("--version").output()
            .map(|o| o.status.success()).unwrap_or(false);
        if docker_ok {
            println!("  \x1b[32m✓\x1b[0m docker available (system cleanup)");
            pass += 1;
        } else {
            println!("  \x1b[33m~\x1b[0m docker not found  \x1b[90m→ optional, needed for Docker cleanup\x1b[0m");
            warn += 1;
        }
    }

    // 9. Status cache
    let cache_path = dirs::config_dir().unwrap_or_default()
        .join("spark").join("repo_status_cache.json");
    if cache_path.exists() {
        let age = std::fs::metadata(&cache_path).ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.elapsed().ok())
            .map(|d| d.as_secs() / 3600);
        match age {
            Some(h) if h < 4 => {
                println!("  \x1b[32m✓\x1b[0m status cache ({}h old, valid)", h);
                pass += 1;
            }
            Some(h) => {
                println!("  \x1b[33m~\x1b[0m status cache ({}h old, expired)  \x1b[90m→ spark status to refresh\x1b[0m", h);
                warn += 1;
            }
            None => {
                println!("  \x1b[33m~\x1b[0m status cache (age unknown)");
                warn += 1;
            }
        }
    } else {
        println!("  \x1b[33m~\x1b[0m no status cache  \x1b[90m→ spark status to create\x1b[0m");
        warn += 1;
    }

    // Summary
    println!("\n  ─────────────────────────────────");
    println!("  {} passed   {} failed   {} warnings", pass, fail, warn);

    if fail == 0 {
        println!("\n  \x1b[32mSpark is healthy!\x1b[0m\n");
    } else {
        println!("\n  Run \x1b[1mspark init\x1b[0m to fix most issues.\n");
    }
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

/// Print repos as a compact table with status indicators
fn print_status_table(statuses: &[(&scanner::repo_manager::ManagedRepo, scanner::repo_manager::RepoStatus)]) {
    use std::collections::BTreeMap;

    struct Entry<'a> {
        owner: &'a str,
        name: &'a str,
        status: &'a scanner::repo_manager::RepoStatus,
        last_commit: &'a Option<String>,
    }

    let mut by_host: BTreeMap<&str, Vec<Entry>> = BTreeMap::new();
    for (r, s) in statuses {
        by_host.entry(&r.host).or_default().push(Entry {
            owner: &r.owner, name: &r.name, status: s, last_commit: &r.last_commit,
        });
    }

    for (host, mut entries) in by_host {
        entries.sort_by(|a, b| a.owner.to_lowercase().cmp(&b.owner.to_lowercase())
            .then(a.name.to_lowercase().cmp(&b.name.to_lowercase())));

        let max_name = entries.iter()
            .map(|e| e.owner.len() + 1 + e.name.len())
            .max().unwrap_or(20) + 2;

        println!("  {} — {} repos\n", host, entries.len());
        for entry in &entries {
            let indicator = match entry.status {
                scanner::repo_manager::RepoStatus::UpToDate => "✓",
                scanner::repo_manager::RepoStatus::Behind(_) => "↓",
                scanner::repo_manager::RepoStatus::Ahead(_) => "↑",
                scanner::repo_manager::RepoStatus::Diverged { .. } => "↕",
                scanner::repo_manager::RepoStatus::Dirty => "●",
                scanner::repo_manager::RepoStatus::Error(_) => "✗",
                scanner::repo_manager::RepoStatus::Checking => "?",
            };
            let repo_name = format!("{}/{}", entry.owner, entry.name);
            let age = entry.last_commit.as_deref().unwrap_or("-");
            let status_str = format!("{}", entry.status);
            println!("  {:<width$}   {}   {:<14}  {}",
                repo_name, indicator, status_str, age, width = max_name);
        }
    }
}

/// Print repos grouped by host/owner as a tree
fn print_repos_tree(repos: &[&scanner::repo_manager::ManagedRepo]) {
    use std::collections::BTreeMap;

    // Group: host -> owner -> [name]
    let mut tree: BTreeMap<&str, BTreeMap<&str, Vec<&str>>> = BTreeMap::new();
    for r in repos {
        tree.entry(&r.host)
            .or_default()
            .entry(&r.owner)
            .or_default()
            .push(&r.name);
    }

    // Sort names within each owner
    for owners in tree.values_mut() {
        for names in owners.values_mut() {
            names.sort_unstable_by_key(|a| a.to_lowercase());
        }
    }

    for (host, owners) in &tree {
        println!("{}", host);
        let owner_count = owners.len();
        for (oi, (owner, names)) in owners.iter().enumerate() {
            let is_last_owner = oi == owner_count - 1;
            let owner_branch = if is_last_owner { "└── " } else { "├── " };
            let owner_prefix = if is_last_owner { "    " } else { "│   " };
            println!("{}{}", owner_branch, owner);
            for (ni, name) in names.iter().enumerate() {
                let is_last_name = ni == names.len() - 1;
                let name_branch = if is_last_name { "└── " } else { "├── " };
                println!("{}{}{}", owner_prefix, name_branch, name);
            }
        }
    }
}
