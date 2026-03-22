# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**SPARK** is a **Rust-based developer operations platform** delivered as a TUI.

It evolved through four generations: Bash scripts (v0.4) -> Go/Bubble Tea TUI (v0.5-v0.6) -> Rust/Ratatui (v0.7) -> DevOps Platform (v0.8). Each version was a revolution, not an increment.

### Four Core Modules

1. **Updater**: Manages updates for 44+ developer tools (AI tools, IDEs, Infrastructure, Runtimes)
2. **Scanner**: Discovers, health-scores, and cleans stale git repos and build artifacts
3. **Repo Manager**: ghq-style repository organizer -- clone, pull, track status across all repos
4. **Port Scanner**: Discovers and kills development servers running on local ports

## Architecture (Rust + Ratatui + tokio)

```
src/
├── main.rs                    # Entry point, terminal setup, tokio runtime
├── app.rs                     # Event loop, background task dispatch via mpsc channels
├── config.rs                  # SparkConfig loaded from ~/.config/spark/config.toml
├── core/
│   ├── types.rs               # Tool, ToolState, Category, UpdateMethod, ToolStatus enums
│   ├── inventory.rs           # 44+ tools catalog with auto-assigned IDs
│   └── changelogs.rs          # Changelog URL mappings with heuristic fallbacks
├── updater/
│   ├── detector.rs            # Version detection (brew, npm, CLI, macOS apps) with async cache
│   ├── version.rs             # Regex-based version parsing + tool-specific parsers
│   └── executor.rs            # Update execution (brew upgrade, npm install -g, curl|sh)
├── scanner/
│   ├── repo_scanner.rs        # Git repo discovery via walkdir + analysis via git2
│   ├── space_analyzer.rs      # Artifact detection (node_modules, venvs, target/, etc.)
│   ├── health.rs              # Health scoring (0-100, grades A-F)
│   ├── cleaner.rs             # Cleanup: trash, archive, delete artifacts
│   ├── repo_manager.rs        # ghq-style clone, pull, status tracking
│   └── port_scanner.rs        # Dev port discovery, runtime detection, process kill
├── tui/
│   ├── model.rs               # App, UpdaterModel, ScannerModel, RepoManagerModel, PortScannerModel
│   ├── update.rs              # Key/message handling, state transitions, Action dispatch
│   ├── scanner_keys.rs        # Scanner/RepoManager/PortScanner key bindings
│   ├── view.rs                # Top-level render dispatcher + tab bar
│   ├── styles.rs              # Color palette, ASCII art, spinner frames
│   └── widgets/
│       ├── splash.rs          # Animated splash screen with color cycling
│       ├── dashboard.rs       # Tool update grid (2 columns, 8 categories) + preview
│       ├── scanner_view.rs    # Repo scan results table + config + cleaning views
│       ├── detail_panel.rs    # Single repo detail view with artifact breakdown
│       ├── repo_manager_view.rs # Repo list, clone input modal, post-clone summary
│       ├── port_view.rs       # Port scanner table with kill actions
│       ├── progress.rs        # Progress bars (determinate + indeterminate) + overlays
│       ├── modal.rs           # Danger zone + clean confirm modals
│       └── search.rs          # Search bar (rendered inline in dashboard)
└── utils/
    ├── shell.rs               # Async shell command execution with timeouts
    └── fs.rs                  # Directory size calculation, git root discovery
```

## Key Concepts

### Async Architecture
- **tokio** runtime for concurrent operations (version checks, filesystem scanning, updates)
- Background tasks communicate with TUI via `tokio::sync::mpsc` unbounded channels
- `AppMessage` enum carries results back to the event loop
- `Action` enum dispatches side effects from key handlers

### State Machines
**Updater**: Splash -> Main -> Search/Preview/Confirm -> Updating -> Summary
**Scanner**: ScanConfig -> Scanning -> ScanResults -> RepoDetail/CleanConfirm -> Cleaning -> CleanSummary
**Repo Manager**: RepoManager -> RepoCloneInput -> RepoCloneSummary -> RepoManager
**Port Scanner**: PortScanner (standalone view)

### Tab Switching
- `TAB` key switches between Updater and Scanner modes
- Scanner mode contains sub-views: scan results, repo manager `[G]`, port scanner `[P]`
- Tab bar always visible at top (except during splash)

### Repo Manager (ghq-style)
- Repos cloned to `{repos_root}/{host}/{owner}/{name}` (configurable in config.toml)
- Post-clone summary shows path, alias suggestion, and AI agent integration tips
- `CloneSummary` struct in model.rs holds summary data for the view

## Development Commands

### Running Locally
```bash
cargo run
```

### Building for Release
```bash
cargo build --release
```

### Adding a New Tool
1. Open `src/core/inventory.rs`
2. Add a new `Tool` struct to the vector
3. If it requires custom version detection, update `src/updater/detector.rs`
4. If it has a known changelog URL, add it to `src/core/changelogs.rs`

## Configuration

Config file: `~/.config/spark/config.toml`

Key fields:
- `scan_directories`: Dirs to scan for git repos
- `stale_threshold_days`: Days to consider a repo stale (default: 90)
- `repos_root`: Root for managed repos (default: `~/repos`)
- `use_trash`: Use OS trash for safe deletions (default: true)
- `max_scan_depth`: Max recursion depth for scanning (default: 4)

See [config.example.toml](config.example.toml) for all options.

## Version History

- **v0.8.0**: **DevOps Platform**. Repo Manager (ghq-style clone/pull/status), Port Scanner, post-clone summary with alias and AI agent tips, configurable `repos_root`.
- **v0.7.0**: **The Rust Migration**. Complete rewrite in Rust + Ratatui. Added Repository Scanner with health scoring, artifact cleanup, and trash-based deletion. Cross-platform support.
- **v0.6.0**: Go/Bubble Tea with 44 tools, 8 categories, full update execution.
- **v0.5.x**: Go/Bubble Tea era. Grid layout, splash screen, danger zone, search.
- **v0.4.x**: Legacy Bash Script era (Archived).

## Legacy Go Code

The original Go implementation is preserved in `cmd/`, `internal/`, `go.mod`, `go.sum` for reference. The active codebase is in `src/` (Rust).
