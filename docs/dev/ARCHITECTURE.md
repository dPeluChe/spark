# SPARK — Architecture

## Overview

SPARK is a **Rust-based developer operations platform** delivered as a TUI (Terminal User Interface), built with **Ratatui** + **tokio**.

**Version**: 0.5.1  
**Language**: Rust  
**Frameworks**: Ratatui (TUI), tokio (async runtime), crossterm (terminal), git2 (git ops)

---

## Project Structure

```
src/
├── main.rs                        # Entry point, terminal setup, tokio runtime
├── config.rs                      # SparkConfig from ~/.config/spark/config.toml
├── app/                           # Event loop + background task spawners
│   ├── mod.rs                     # run(): draw, poll events, dispatch, tick
│   ├── actions.rs                 # dispatch_action per Action variant
│   └── spawn.rs                   # Updater spawn helpers (version checks, update)
├── core/
│   ├── types.rs                   # Tool, Category, UpdateMethod, ToolStatus enums
│   ├── changelogs.rs              # Changelog URL mappings + heuristic fallbacks
│   └── inventory/                 # 55+ tools catalog (auto-assigned S-## IDs)
│       ├── mod.rs                 # get_inventory() concatenates + assigns IDs
│       ├── dev.rs                 # System + AI code tools + IDEs + terminals
│       └── platform.rs            # Productivity + infra + runtimes + utilities
├── updater/
│   ├── detector.rs                # Async version detection (brew/npm cache warmup)
│   ├── version.rs                 # Regex-based version parsing
│   └── executor.rs                # Update execution with 10-minute timeout
├── scanner/
│   ├── mod.rs                     # Module exports
│   ├── common.rs                  # Shared helpers (shorten_path, redact, ignore)
│   ├── repo_scanner.rs            # Git repo discovery + analysis via git2
│   ├── space_analyzer.rs          # Artifact detection (20+ types)
│   ├── health.rs                  # Health scoring (0-100, grades A-F)
│   ├── cleaner.rs                 # Trash-based or permanent deletion
│   ├── repo_manager.rs            # ghq-style clone/pull/status + 4h cache
│   ├── repo_tags.rs               # Persistent multi-tag system
│   ├── repo_ingest.rs             # Thin wrapper over trs ingest (see TRS_INTEGRATION.md)
│   ├── system_cleaner.rs          # Docker/caches/VMs/logs + safety guards
│   ├── system_categories.rs       # Category definitions + risk levels
│   ├── history_scanner.rs         # Git commit diff secrets via git2
│   ├── cert_scanner.rs            # SSL/TLS cert scan (x509-parser + Keychain)
│   ├── secret_scanner/            # Secrets + credentials detection
│   │   ├── mod.rs                 # Types + scan_directory + integration tests
│   │   ├── patterns.rs            # Regex statics + SENSITIVE_FILES/EXTENSIONS
│   │   ├── context.rs             # detect_context (test/doc/config classifier)
│   │   ├── filename.rs            # Sensitive filename + .env + credential configs
│   │   └── content.rs             # Content scan (API keys, URLs, generic secrets)
│   ├── code_patterns/             # OWASP Top 10:2025 pattern detection
│   │   ├── mod.rs                 # Types + scan + classify_file + tests
│   │   └── patterns.rs            # Regex statics + PATTERNS catalog
│   ├── dep_scanner/               # Dependency vulnerabilities
│   │   ├── mod.rs                 # Types + OSV.dev query + severity ordering
│   │   └── parsers.rs             # package.json/lock, requirements.txt, Cargo.toml/lock
│   └── port_scanner/              # Listening TCP ports + process info
│       ├── mod.rs                 # Types, scan dispatcher, kill_process + tests
│       ├── macos.rs               # lsof-based + batched ps/lsof metadata
│       ├── linux.rs               # /proc/net/tcp + /proc/<pid>/fd inode matching
│       └── runtime.rs             # Process/cmdline → Runtime, project dir resolution
├── tui/
│   ├── view.rs                    # Top-level render dispatcher + tab bar
│   ├── styles.rs                  # Color palette, spinner frames, modal helpers
│   ├── model/                     # All application state
│   │   ├── mod.rs                 # Enums (AppMode, ScannerState, …) + Toast + App
│   │   ├── updater.rs             # UpdaterModel
│   │   ├── scanner.rs             # ScannerModel + PortScannerModel
│   │   ├── repo.rs                # RepoManagerModel + CloneSummary
│   │   ├── system.rs              # SystemCleanerModel
│   │   └── audit.rs               # AuditModel
│   ├── update/                    # Event dispatch: keys → Action, messages → state
│   │   ├── mod.rs                 # Action enum + handle_key + welcome + tab cycle
│   │   ├── messages.rs            # handle_message (AppMessage → state update)
│   │   └── updater_keys.rs        # Updater tab key bindings
│   ├── scanner_keys/              # Key bindings for non-updater tabs
│   │   ├── mod.rs                 # Dispatcher
│   │   ├── scanner_tab.rs         # Scanner/container/clean/delete keys
│   │   ├── repo_tab.rs            # Repo manager keys
│   │   ├── port_tab.rs            # Port scanner keys
│   │   ├── system_tab.rs          # System cleanup keys
│   │   └── audit_tab.rs           # Security audit keys
│   └── widgets/                   # Pure rendering — splash, dashboard, progress,
│       │                          # modal, detail_panel, port_view, …
│       ├── scanner_view/          # mod.rs + config.rs + results.rs
│       ├── repo_manager_view/     # mod.rs + clone.rs
│       ├── audit_view/            # mod.rs + detail.rs + deps.rs
│       └── system_view/           # mod.rs + table.rs + risk.rs
├── cli/
│   ├── mod.rs                     # CLI definitions (clap), dispatcher, shared helpers
│   ├── system.rs                  # init, config, doctor, agent, completions, root
│   ├── certs.rs                   # Certificate scanner CLI
│   ├── tags.rs                    # Tag management (add, remove, list, delete, rename)
│   ├── ingest.rs                  # spark ingest — LLM-ready context digest
│   ├── repos/                     # Repository management CLI
│   │   ├── mod.rs                 # clone, list, search, cd, rm + tree print
│   │   ├── status.rs              # spark status (cached + fresh check)
│   │   └── pull.rs                # spark pull (single / all / tag)
│   ├── ports/                     # spark ps — unified process + port inspector
│   │   ├── mod.rs                 # Dispatcher by query/kill combinations
│   │   ├── list.rs                # List ports grouped into dev/macOS/services/apps
│   │   ├── search.rs              # Search processes by name + cross-ref ports
│   │   ├── kill.rs                # Interactive + silent kill by port/PID/name
│   │   └── ps_list.rs             # `ps aux` → structured PsEntry
│   └── audit/                     # Security audit CLI
│       ├── mod.rs                 # cmd_audit orchestration + phase runners + summary
│       ├── secrets.rs             # Secrets & credentials render
│       ├── history.rs             # Git history render
│       ├── patterns.rs            # OWASP Top 10 render
│       ├── deps.rs                # OSV.dev + npm audit render + `audit --deps`
│       └── ignore.rs              # .sparkauditignore scaffold
└── utils/
    ├── mod.rs                     # Module exports
    ├── shell.rs                   # Async Command wrapper with timeout + debug log
    └── fs.rs                      # dir_size, format_size, shorten_path, expand_tilde
```

---

## Architectural Layers

### 1. Entry Point (`main.rs`)

Sets up the tokio runtime, initializes crossterm alternate screen, loads `SparkConfig`, and launches the event loop in `app.rs`. Non-TUI commands (`spark clone`, `spark audit`, etc.) are handled directly from `main.rs` via clap and exit early.

### 2. Event Loop (`app.rs`)

The central orchestrator:
- Polls crossterm events at 100ms tick rate
- Delegates key events to `scanner_keys/` handler which returns `Option<Action>`
- Dispatches `Action` variants as background tokio tasks
- Receives `AppMessage` results via `mpsc::unbounded_channel`
- Calls `view::draw()` each frame via Ratatui

### 3. Core Domain (`core/`)

- **types.rs**: Enums for `UpdateMethod`, `Category`, `ToolStatus`, and structs `Tool`, `ToolState`
- **inventory.rs**: Static catalog of 55+ tools with auto-assigned IDs (`S-01`, `S-02`, ...)
- **changelogs.rs**: Maps tool names to changelog URLs with heuristic fallbacks for brew/npm

### 4. Updater (`updater/`)

- **detector.rs**: Async version detection with brew/npm outdated cache warmup
- **version.rs**: Regex-based parsing for semver, major.minor, date versions, git hashes, and tool-specific formats
- **executor.rs**: Executes updates via `tokio::process::Command` with 10-minute timeout

### 5. Scanner (`scanner/`)

The data layer — all scanning, analysis, and mutation:

- **common.rs**: Shared path helpers (`shorten_path`, `format_path`) used across scanner modules
- **repo_scanner.rs**: Walks directories to find `.git` repos; analyzes via `git2` (branch, commits, dirty status, remotes)
- **health.rs**: Scores repos 0-100 based on commit recency, remote presence, dirty state, artifact size
- **space_analyzer.rs**: Detects 20+ artifact types (node_modules, venvs, target/, .gradle, etc.)
- **cleaner.rs**: Trash-based or permanent deletion of artifacts/repos
- **repo_manager.rs**: ghq-style clone to `{root}/{host}/{owner}/{name}`, pull, status checks, 4h cache
- **repo_tags.rs**: Persistent tagging system — repos can have multiple tags, stored in config
- **repo_ingest.rs**: Thin fleet-level wrapper over `trs ingest`. TRS owns generation + storage (`~/.trs/ingest/`, shared). SPARK adds batch mode (`--all` with `trs --fresh`) and fleet-aware listing. See [TRS_INTEGRATION.md](TRS_INTEGRATION.md).
- **port_scanner.rs**: Batched `lsof`/`ps` on macOS, `/proc/net/tcp` on Linux; detects runtime (Node, Python, Go, Rust, etc.)
- **system_cleaner.rs**: Docker, dev caches, VMs, logs — with path blocklist, app-aware checks, operation log
- **system_categories.rs**: Category definitions and risk levels for system cleanup items
- **secret_scanner.rs**: Regex-based detection of API keys, credentials, sensitive files with context-aware severity and `.sparkauditignore` support
- **history_scanner.rs**: Walks git commit diffs via git2 to find secrets in past commits (reuses patterns from secret_scanner)
- **code_patterns.rs**: OWASP Top 10:2025 patterns — SQL injection, command injection, XSS, insecure crypto, deserialization, config, path traversal
- **dep_scanner.rs**: Parses package.json/lock, requirements.txt, Cargo.toml/lock; queries OSV.dev batch API for known vulnerabilities
- **cert_scanner.rs**: SSL/TLS certificate parsing with x509-parser (pure Rust, no openssl), macOS Keychain scan via `security find-certificate`, home directory key/cert file discovery

### 6. TUI (`tui/`)

MVU (Model-View-Update) pattern:

1. Key event → `scanner_keys/` handler returns `Option<Action>`
2. Action dispatched via `mpsc` channel to `app.rs`
3. `app.rs` spawns async tasks, sends `AppMessage` back
4. Model updates, next frame renders

- **model.rs**: All application state — `App` holds `UpdaterModel`, `ScannerModel`, `PortScannerModel`, `RepoManagerModel`, `AuditModel`
- **update.rs**: Handles `AppMessage` variants, updates model, returns `Action` enum for side effects
- **scanner_keys/**: Key bindings split by tab — each file handles one tab's key events
- **view.rs** + **widgets/**: Pure rendering functions; widgets handle their own layout; no state mutation

### 7. CLI (`cli/`)

All `spark <subcommand>` implementations. Each file maps to one command group. Shared formatting helpers in `mod.rs`.

### 8. Utils (`utils/`)

- **shell.rs**: Async `tokio::process::Command` wrapper with timeout, stderr capture, debug logging to `spark_debug.log`
- **fs.rs**: `dir_size`, `format_size`, `shorten_path` (home → `~`), `expand_tilde` (handles `~/` paths)

---

## State Machines

### Updater
```
Splash → Main → Search / Preview / Confirm → Updating → Summary → Main
```

### Scanner
```
ScanConfig → Scanning → ScanResults → RepoDetail
                                    → ContainerChildDetail / ContainerChildDelete
                                    → CleanConfirm → Cleaning
                                    → ScanAddPath
```

### Repo Manager
```
RepoManager → RepoCloneInput → RepoCloneSummary → RepoManager
```

### Port Scanner
```
PortScan → PortKillConfirm → PortScan
```

### System Cleanup
```
SystemMain → SystemCleanConfirm → SystemMain
           → SystemCleanConfirmBulk → SystemMain
```

### Security Audit
```
SecretAudit → SecretAuditPathInput → SecretAudit
            → SecretAuditRunning → SecretAuditResults → SecretAuditDetail
```

---

## Concurrency Model

- **tokio runtime**: All I/O runs as spawned tasks; UI thread never blocks
- **mpsc channels**: Background tasks send `AppMessage` back to the event loop
- **Action dispatch**: Key handlers return `Action` enums; `app.rs` spawns the corresponding task
- Version checks run in parallel (~55 concurrent tasks per detect cycle)
- Scanner uses progress reporting via a secondary mpsc channel
- Bulk system cleanup spawns one `spawn_blocking` task per selected item

---

## Design Patterns

1. **Message-passing**: TUI state is only modified in the event loop via `AppMessage` — no shared mutable state
2. **Strategy pattern**: `UpdateMethod` enum drives detection and execution behavior per tool
3. **State machine**: Each module has explicit states (`ScannerState`) with validated transitions
4. **Action/Command pattern**: Key handlers return `Action`, event loop executes side effects
5. **Pure rendering**: All widget functions are pure — they read state and produce `Frame` calls; no mutation

---

## Testing

```bash
cargo test    # 127 tests
```

Tests cover: version parsing, health scoring, config serialization/deserialization, inventory validation, changelog URL mapping, artifact detection, port detection, git URL parsing, path utilities, and TUI model logic. Tests live next to the code they test (`#[cfg(test)]` at the bottom of each file).

---

## Dependencies

### Core
- `ratatui` — TUI framework
- `crossterm` — Terminal manipulation
- `tokio` — Async runtime
- `git2` — Git operations (libgit2 bindings)

### Scanner / Security
- `walkdir` — Filesystem traversal
- `x509-parser` — Certificate parsing (pure Rust, no openssl)
- `reqwest` — HTTP client (OSV.dev API)

### Utilities
- `serde` / `serde_json` / `toml` — Config + OSV/npm-audit JSON serialization
- `regex` / `once_cell` — Pattern matching and lazy statics
- `chrono` — Date/time for health scoring
- `color-eyre` — Error reporting
- `dirs` — Platform-specific directory paths (home, config, data)
- `clap` / `clap_complete` — CLI argument parsing + shell completions
- `walkdir` — Filesystem traversal
