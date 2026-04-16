# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**SPARK** is a **Rust-based developer operations platform** delivered as a TUI.

The active codebase is 100% Rust. It manages repos, scans health, cleans artifacts, monitors ports, cleans system caches, audits security, and updates dev tools.

### Six Core Modules

1. **Scanner**: Discovers, health-scores, and cleans stale git repos and build artifacts
2. **Repo Manager**: ghq-style repository organizer — clone, pull, track status across all repos
3. **Port Scanner**: Discovers and kills development servers running on local ports
4. **System Cleanup**: Docker, dev caches (brew/npm/pip/cargo), VMs, logs — with safety guards
5. **Security Audit**: Secrets, git history, OWASP code patterns, dependency vulnerabilities (OSV.dev + npm audit)
6. **Updater**: Manages updates for 55 developer tools (AI tools, IDEs, Infrastructure, Runtimes)

### CLI Commands

```bash
spark                      # TUI
spark init                 # Setup shell + completions
spark clone <url>          # Clone (ghq-compatible)
spark cd <name>            # Find repo path
spark search <query>       # Search repos (shows status, commit age, path)
spark list [-p] [query]    # List repos (tree view by host/owner)
spark status [query]       # Check which repos need pull (fetch + compare)
spark status --tag <tag>   # Check status of repos with a specific tag
spark pull <query|all>     # Pull repos behind remote (ff-only)
spark pull all --tag <t>   # Pull all repos with a specific tag
spark tag add <repo> <tag> # Tag a repo (repos can have multiple tags)
spark tag remove <repo> <t># Remove tag from repo
spark tag list [tag]       # List all tags or repos in a tag
spark tag delete <tag>     # Delete entire tag
spark tag rename <old> <n> # Rename a tag
spark audit [path]         # Security audit (secrets + history + OWASP + deps)
spark audit --deps         # Dependency-only scan (OSV.dev + npm audit)
spark audit --offline      # Local-only scan (no network)
spark audit --init         # Create .sparkauditignore
spark audit -o report.txt  # Save audit report to file
spark ps                   # Dev server ports (pid, process, runtime, project)
spark ps --all             # All ports: macOS / SERVICES / APPS sections
spark ps <query>           # Search processes by name, cross-ref with ports
spark ps --kill <target>   # Kill by port, PID, or name (interactive)
spark ps <query> --kill    # Kill non-interactive (exit 0/1 — for scripts/agents)
spark certs                # Scan certificates (Keychain + files + ~/home)
spark certs --keychain     # Keychain only
spark certs --expired      # Show only expired
spark certs --summary      # Summary without details
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
├── main.rs                    # Entry point, terminal setup
├── app.rs                     # Event loop, action dispatch via mpsc channels
├── config.rs                  # SparkConfig + ghq root detection
├── cli/
│   ├── mod.rs                 # CLI definitions (clap), dispatcher, shared helpers
│   ├── repos.rs               # clone, list, search, cd, rm, status, pull
│   ├── audit.rs               # audit command (secrets, history, OWASP, deps output)
│   └── system.rs              # init, config, doctor, agent, completions, root
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
│   ├── port_scanner.rs        # Port discovery (batched lsof macOS, /proc Linux)
│   ├── system_cleaner.rs      # Docker, caches, VMs, logs cleanup + safety
│   ├── secret_scanner.rs      # API keys, credentials, sensitive files, .env detection
│   ├── history_scanner.rs     # Git commit history scan via git2
│   ├── code_patterns.rs       # OWASP Top 10:2025 pattern detection
│   ├── dep_scanner.rs         # Dependency vulnerabilities (OSV.dev API + npm audit)
│   ├── cert_scanner.rs        # SSL/TLS certificate scanning (x509-parser + macOS Keychain)
│   └── repo_tags.rs           # Repository tagging/grouping system
├── tui/
│   ├── model.rs               # All state models + Toast notifications
│   ├── update.rs              # Key/message handling, Action dispatch
│   ├── scanner_keys/           # Key bindings split by tab
│   │   ├── mod.rs             # Dispatcher
│   │   ├── scanner_tab.rs     # Scanner/container/clean/delete keys
│   │   ├── repo_tab.rs        # Repo manager keys
│   │   ├── port_tab.rs        # Port scanner keys
│   │   ├── system_tab.rs      # System cleanup keys
│   │   └── audit_tab.rs       # Security audit keys
│   ├── view.rs                # Tab bar + render dispatcher
│   ├── styles.rs              # Color palette, ASCII art
│   └── widgets/               # splash, dashboard, scanner_view, detail_panel,
│                              # repo_manager_view, port_view, system_view,
│                              # audit_view, progress, modal
├── cli/
│   ├── mod.rs                 # CLI definitions (clap), dispatcher, shared helpers
│   ├── repos.rs               # clone, list, search, cd, rm, status, pull
│   ├── audit.rs               # audit command output (4 phases)
│   ├── certs.rs               # certificate scanner CLI
│   ├── tags.rs                # tag management CLI (add, remove, list, delete, rename)
│   └── system.rs              # init, config, doctor, agent, completions, root
└── utils/
    ├── shell.rs               # Async commands + debug logging
    └── fs.rs                  # dir_size, format_size, shorten_path, safe_truncate
```

### Certificate Scanner (`spark certs`)
- Parses `.pem`, `.crt`, `.cer` files with `x509-parser` (pure Rust, no openssl dependency)
- macOS Keychain scan via `security find-certificate`
- Home directory scan for loose key/cert files (SSH keys, private keys, stale certs)
- Grouped by issuer with oldest/newest range for large groups
- Context-aware recommendations: Apple (safe to remove), Developer (renew in Xcode), Self-signed (review and rotate)
- Sections: Expired → Expiring → Valid (all shown by default)

## Key Concepts

### Tab Navigation
`TAB` cycles: Scanner → Repos → Ports → System → Audit → Updater
`q` goes back (not quit) in sub-views. Only quits at root level.

### Security Audit (4 phases)
1. **Secrets scan**: API keys, credentials, sensitive files, .env detection with context-aware severity
2. **Git history scan**: Walks commit diffs via git2 for secrets in past commits
3. **Code patterns (OWASP Top 10:2025)**: SQL injection, command injection, XSS, insecure crypto, deserialization, config, path traversal
4. **Dependency scan**: Queries OSV.dev batch API + runs npm audit if package-lock.json exists

Context-aware severity: Source Code > Config > Test > Docs (findings in tests/docs downgraded to info)
`.sparkauditignore`: gitignore-style file to suppress reviewed findings

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
cargo test                 # 124 tests
cargo build --release      # Optimized build
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
