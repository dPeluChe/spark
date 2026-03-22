mod app;
mod config;
mod core;
mod updater;
mod scanner;
mod tui;
mod utils;

use std::io;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;

/// SPARK — Surgical Precision Update Utility & Repository Scanner
#[derive(Parser)]
#[command(name = "spark", version, about, long_about = None)]
struct Cli {
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
    scan_dir: Option<Vec<std::path::PathBuf>>,

    /// Override max scan depth
    #[arg(long)]
    max_depth: Option<usize>,
}

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    // Build config with CLI overrides
    let mut config = config::SparkConfig::load();
    if let Some(dirs) = cli.scan_dir {
        config.scan_directories = dirs;
    }
    if let Some(depth) = cli.max_depth {
        config.max_scan_depth = depth;
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run app
    let result = app::run(
        &mut terminal,
        config,
        cli.scan_only,
        cli.update_only,
        cli.dry_run,
    )
    .await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if result.is_ok() {
        println!("\n  See you later, Space Cowboy...");
        println!("  Spark sequence complete.\n");
    }

    result
}
