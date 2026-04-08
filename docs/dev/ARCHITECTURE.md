# SPARK - Architecture Documentation

## Overview

SPARK is a **Rust-based DevOps platform** delivered as a TUI (Terminal User Interface), built with **Ratatui** + **tokio**.

**Version**: 0.8.0
**Language**: Rust
**Frameworks**: Ratatui (TUI), tokio (async runtime), crossterm (terminal), git2 (git ops)

---

## Project Structure

```
src/
├── main.rs                     # Entry point, terminal setup, tokio runtime
├── app.rs                      # Event loop, action dispatch via mpsc channels
├── config.rs                   # SparkConfig from ~/.config/spark/config.toml
├── core/
│   ├── types.rs                # Tool, Category, UpdateMethod, ToolStatus
│   ├── inventory.rs            # 44+ tools catalog with auto-assigned IDs
│   └── changelogs.rs           # Changelog URL mappings with heuristic fallbacks
├── updater/
│   ├── detector.rs             # Version detection (brew, npm, CLI, macOS apps) with async cache
│   ├── version.rs              # Regex-based version parsing + tool-specific parsers
│   └── executor.rs             # Update execution (brew upgrade, npm install -g, curl|sh)
├── scanner/
│   ├── repo_scanner.rs         # Git repo discovery via walkdir + analysis via git2
│   ├── space_analyzer.rs       # Artifact detection (node_modules, venvs, target/, etc.)
│   ├── health.rs               # Health scoring (0-100, grades A-F)
│   ├── cleaner.rs              # Cleanup: trash, archive, delete artifacts
│   ├── repo_manager.rs         # ghq-style clone, pull, status tracking
│   ├── port_scanner.rs         # Dev port discovery, runtime detection, process kill
│   ├── system_cleaner.rs       # Docker, caches, VMs, logs cleanup + safety guards
│   ├── secret_scanner.rs       # API keys, credentials, sensitive files detection
│   ├── history_scanner.rs      # Git commit history scan via git2
│   ├── code_patterns.rs        # OWASP Top 10:2025 pattern detection
│   └── dep_scanner.rs          # Dependency vulnerabilities (OSV.dev + npm audit)
├── tui/
│   ├── model.rs                # App, UpdaterModel, ScannerModel, RepoManagerModel, PortScannerModel
│   ├── update.rs               # Key/message handling, state transitions, Action dispatch
│   ├── scanner_keys.rs         # Scanner/RepoManager/PortScanner key bindings
│   ├── view.rs                 # Top-level render dispatcher + tab bar
│   ├── styles.rs               # Color palette, ASCII art, spinner frames
│   └── widgets/                # UI components (splash, dashboard, scanner, audit_view, modals, etc.)
└── utils/
    ├── shell.rs                # Async shell command execution with timeouts
    └── fs.rs                   # Directory size calculation, format_size
```

---

## Architectural Layers

### 1. Entry Point (`main.rs`)

Sets up the tokio runtime, initializes crossterm alternate screen, loads `SparkConfig`, and launches the event loop in `app.rs`.

### 2. Event Loop (`app.rs`)

The central orchestrator:
- Polls crossterm events at 100ms tick rate
- Delegates key events to `update::handle_key()` which returns `Option<Action>`
- Dispatches `Action` variants as background tokio tasks
- Receives `AppMessage` results via `mpsc::unbounded_channel`
- Calls `view::draw()` each frame via Ratatui

### 3. Core Domain (`core/`)

- **types.rs**: Enums for `UpdateMethod`, `Category`, `ToolStatus`, and structs `Tool`, `ToolState`
- **inventory.rs**: Static catalog of 44+ tools with auto-assigned IDs (`S-01`, `S-02`, ...)
- **changelogs.rs**: Maps tool names to changelog URLs with heuristic fallbacks for brew/npm

### 4. Updater (`updater/`)

- **detector.rs**: Async version detection with brew/npm outdated cache warmup
- **version.rs**: Regex-based parsing for semver, major.minor, date versions, git hashes, and tool-specific formats
- **executor.rs**: Executes updates via `tokio::process::Command` with 10min timeout

### 5. Scanner (`scanner/`)

- **repo_scanner.rs**: Walks directories to find `.git` repos, analyzes via `git2` (branch, commits, dirty status, remotes)
- **health.rs**: Scores repos 0-100 based on commit recency, remote presence, dirty state, artifact size
- **space_analyzer.rs**: Detects 20+ artifact types (node_modules, venvs, target/, .gradle, etc.)
- **cleaner.rs**: Trash-based or permanent deletion of artifacts/repos
- **repo_manager.rs**: ghq-style clone to `{root}/{host}/{owner}/{name}`, pull, status checks, 4h cache
- **port_scanner.rs**: Batched `lsof`/`ps` on macOS, `/proc/net/tcp` on Linux; detects runtime (Node, Python, Go, Rust, etc.)
- **secret_scanner.rs**: Regex-based detection of API keys, credentials, sensitive files with context-aware severity and `.sparkauditignore` support
- **history_scanner.rs**: Walks git commit diffs via git2 to find secrets in past commits (reuses patterns from secret_scanner)
- **code_patterns.rs**: OWASP Top 10:2025 patterns — SQL injection, command injection, XSS, insecure crypto, deserialization, config, path traversal
- **dep_scanner.rs**: Parses package.json/lock, requirements.txt, Cargo.toml/lock; queries OSV.dev batch API for known vulnerabilities
- **cert_scanner.rs**: SSL/TLS certificate parsing with x509-parser, macOS Keychain scan, home directory key/cert file discovery

### 6. TUI (`tui/`)

- **model.rs**: All application state - `App` holds `UpdaterModel`, `ScannerModel`, `PortScannerModel`, `RepoManagerModel`
- **update.rs**: Key and message handlers, returns `Action` enum for side effects
- **view.rs** + **widgets/**: Pure rendering functions, each widget handles its own layout

---

## State Machines

### Updater
```
Splash -> Main -> Search/Preview/Confirm -> Updating -> Summary -> Main
```

### Scanner
```
ScanConfig -> Scanning -> ScanResults -> RepoDetail -> ContainerChildDetail/ContainerChildDelete
                                      -> CleanConfirm -> Cleaning
```

### Repo Manager
```
RepoManager -> RepoCloneInput -> RepoCloneSummary -> RepoManager
```

### Port Scanner
```
PortScan -> PortKillConfirm -> PortScan
```

---

## Concurrency Model

- **tokio runtime**: All I/O runs as spawned tasks
- **mpsc channels**: Background tasks send `AppMessage` back to the event loop
- **Action dispatch**: Key handlers return `Action` enums; `app.rs` spawns the corresponding task
- Version checks run in parallel (~44 concurrent tasks)
- Scanner uses progress reporting via a secondary mpsc channel

---

## Design Patterns

1. **Message-passing architecture**: TUI state is only modified in the event loop via `AppMessage`
2. **Strategy pattern**: `UpdateMethod` enum drives detection and execution behavior
3. **State machine**: Each module has explicit states with validated transitions
4. **Action/Command pattern**: Key handlers return `Action`, event loop executes side effects

---

## Testing

```bash
cargo test    # 118 tests
```

Tests cover: version parsing, health scoring, config serialization/deserialization, inventory validation, changelog URL mapping, artifact detection, port detection, git URL parsing, and TUI model logic.

---

## Dependencies

### Core
- `ratatui` - TUI framework
- `crossterm` - Terminal manipulation
- `tokio` - Async runtime
- `git2` - Git operations (libgit2 bindings)

### Utilities
- `serde` / `toml` - Config serialization
- `regex` / `once_cell` - Version parsing
- `walkdir` - Filesystem traversal
- `chrono` - Date/time for health scoring
- `reqwest` - HTTP client (OSV.dev API queries)
- `x509-parser` - Certificate parsing (pure Rust)
- `color-eyre` - Error reporting
- `dirs` - Platform-specific directories
