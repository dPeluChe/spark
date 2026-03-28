# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**SPARK** is a **Rust-based developer operations platform** delivered as a TUI.

The active codebase is 100% Rust. It manages repos, scans health, cleans artifacts, monitors ports, cleans system caches, and updates dev tools.

### Five Core Modules

1. **Scanner**: Discovers, health-scores, and cleans stale git repos and build artifacts
2. **Repo Manager**: ghq-style repository organizer — clone, pull, track status across all repos
3. **Port Scanner**: Discovers and kills development servers running on local ports
4. **System Cleanup**: Docker, dev caches (brew/npm/pip/cargo), VMs, logs — with safety guards
5. **Updater**: Manages updates for 44+ developer tools (AI tools, IDEs, Infrastructure, Runtimes)

### CLI Commands

```bash
spark                      # TUI
spark init                 # Setup shell + completions
spark clone <url>          # Clone (ghq-compatible)
spark cd <name>            # Find repo path
spark search <query>       # Search repos (shows status, commit age, path)
spark list [-p] [query]    # List repos (tree view by host/owner)
spark status [query]       # Check which repos need pull (fetch + compare)
spark pull <query|all>     # Pull repos behind remote (ff-only)
spark root [--set]         # Show/change root
spark rm <query>           # Remove repo
spark config               # Show/update config
spark agent                # AI agent tips
spark doctor               # Validate installation health
spark completions <shell>  # Shell completions
```

## Architecture (Rust + Ratatui + tokio)

```
src/
├── main.rs                    # Entry point, CLI (clap), terminal setup
├── app.rs                     # Event loop, action dispatch via mpsc channels
├── config.rs                  # SparkConfig + ghq root detection
├── core/
│   ├── types.rs               # Tool, ToolState, Category, UpdateMethod enums
│   ├── inventory.rs           # 44+ tools catalog
│   └── changelogs.rs          # Changelog URL mappings
├── updater/
│   ├── detector.rs            # Version detection (brew, npm, CLI, macOS apps)
│   ├── version.rs             # Regex-based version parsing
│   └── executor.rs            # Update execution
├── scanner/
│   ├── repo_scanner.rs        # Git repo discovery + analysis via git2
│   ├── space_analyzer.rs      # Artifact detection (20+ types)
│   ├── health.rs              # Health scoring (0-100, grades A-F)
│   ├── cleaner.rs             # Artifact cleanup (trash or delete)
│   ├── repo_manager.rs        # ghq-style clone/pull/status + cache
│   ├── port_scanner.rs        # Port discovery (lsof macOS, /proc Linux)
│   └── system_cleaner.rs      # Docker, caches, VMs, logs cleanup + safety
├── tui/
│   ├── model.rs               # All state models + Toast notifications
│   ├── update.rs              # Key/message handling, Action dispatch
│   ├── scanner_keys.rs        # Scanner/Repos/Ports/System key bindings
│   ├── view.rs                # Tab bar + render dispatcher
│   ├── styles.rs              # Color palette, ASCII art
│   └── widgets/               # splash, dashboard, scanner_view, detail_panel,
│                              # repo_manager_view, port_view, system_view,
│                              # progress, modal
└── utils/
    ├── shell.rs               # Async commands + debug logging
    └── fs.rs                  # dir_size, format_size
```

## Key Concepts

### Tab Navigation
`TAB` cycles: Scanner → Repos → Ports → System → Updater
`q` goes back (not quit) in sub-views. Only quits at root level.

### Safety (System Cleanup)
- Path validation against protected system paths
- App-aware: `pgrep` checks before cleaning app caches
- Age-based: logs >7 days only
- Operation log: `~/.config/spark/operations.log`
- Whitelist: `~/.config/spark/whitelist.txt`
- Dry-run: `spark --dry-run`

### Repo Status Cache
- Stored in `repo_status_cache.json`
- Expires after 4 hours
- Sequential fetch (not parallel) to avoid network overload
- `r` in Repos clears cache and re-fetches

## Development

```bash
cargo run                  # Dev mode
cargo test                 # 79 tests
cargo build --release      # Optimized build (~2.7MB)
```

## Configuration

Config: `~/.config/spark/config.toml` (macOS: `~/Library/Application Support/spark/`)

Key fields:
- `repos_root`: Root for managed repos (auto-detects ghq root)
- `max_scan_depth`: Recursion depth for scanning (default: 6)
- `stale_threshold_days`: Days to consider stale (default: 90)
- `use_trash`: Use trash for deletions (default: true)

## Distribution

```bash
# npm
npm install -g @dpeluche/spark

# cargo
cargo install --git https://github.com/dPeluChe/labs-spark

# source
./install.sh && spark init
```
