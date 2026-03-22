# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**SPARK** is a **Rust-based TUI application** (migrated from Go/Bubble Tea in v0.7.0).
It has two main modes:
1. **Updater**: Manages system updates for 44+ developer tools (AI tools, IDEs, Infrastructure)
2. **Scanner**: Discovers, analyzes, and cleans stale git repositories and build artifacts

## Architecture (Rust + Ratatui)

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
│   └── cleaner.rs             # Cleanup: trash, archive, delete artifacts
├── tui/
│   ├── model.rs               # App, UpdaterModel, ScannerModel state structs
│   ├── update.rs              # Key/message handling, state transitions, Action dispatch
│   ├── view.rs                # Top-level render dispatcher + tab bar
│   ├── styles.rs              # Color palette, ASCII art, spinner frames
│   └── widgets/
│       ├── splash.rs          # Animated splash screen with color cycling
│       ├── dashboard.rs       # Tool update grid (2 columns, 8 categories) + preview
│       ├── scanner_view.rs    # Repo scan results table + config + cleaning views
│       ├── detail_panel.rs    # Single repo detail view with artifact breakdown
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
**Updater**: Splash → Main → Search/Preview/Confirm → Updating → Summary
**Scanner**: ScanConfig → Scanning → ScanResults → RepoDetail/CleanConfirm → Cleaning → CleanSummary

### Tab Switching
- `TAB` key switches between Updater and Scanner modes
- Tab bar always visible at top (except during splash)

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

## Version History

- **v0.7.0**: **The Rust Migration**. Complete rewrite in Rust + Ratatui. Added Repository Scanner with health scoring, artifact cleanup, and trash-based deletion. Cross-platform support.
- **v0.6.0**: Go/Bubble Tea with 44 tools, 8 categories, full update execution.
- **v0.5.x**: Go/Bubble Tea era. Grid layout, splash screen, danger zone, search.
- **v0.4.x**: Legacy Bash Script era (Archived).

## Legacy Go Code

The original Go implementation is preserved in `cmd/`, `internal/`, `go.mod`, `go.sum` for reference. The active codebase is in `src/` (Rust).
