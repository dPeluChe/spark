<p align="center">
  <strong>SPARK</strong> — developer operations platform
</p>

<p align="center">
  <a href="https://dpeluche.github.io/spark/"><strong>dpeluche.github.io/spark</strong></a> ·
  <a href="https://github.com/dPeluChe/spark">GitHub</a> ·
  <a href="https://www.npmjs.com/package/@dpeluche/spark">npm</a> ·
  <a href="README.es.md">Español</a>
</p>

<p align="center">
  <a href="https://github.com/dPeluChe/spark/actions"><img src="https://github.com/dPeluChe/spark/actions/workflows/release.yml/badge.svg" alt="Release"></a>
  <a href="https://github.com/dPeluChe/spark/releases"><img src="https://img.shields.io/github/v/release/dPeluChe/spark" alt="Release"></a>
  <a href="https://www.npmjs.com/package/@dpeluche/spark"><img src="https://img.shields.io/npm/v/@dpeluche/spark" alt="npm"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License"></a>
</p>

<p align="center">
  <a href="#why">Why</a> ·
  <a href="#install">Install</a> ·
  <a href="#modules">Modules</a> ·
  <a href="#cli">CLI</a> ·
  <a href="#keyboard-controls">Keys</a> ·
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

---

## Why

SPARK started as a personal tool. Managing dozens of git repos, watching build caches balloon, forgetting which dev servers are still running, and checking SSL cert expiration dates across multiple projects — it was all scattered across different tools, scripts, and browser tabs.

We wanted one terminal interface that could handle the full developer operations loop: scan repo health, clean stale artifacts, track remote status across all repos, monitor running processes, audit security, and keep dev tools updated. So we built it in Rust.

The landing page has the full write-up: <https://dpeluche.github.io/spark/>

## What it looks like

```
┌─ SPARK v0.5.1 ────────────────────────────────────────────────────────────┐
│  Scanner   Repos   Ports   System   Audit   Updater                       │
├────────────────────────────────────────────────────────────────────────────┤
│  REPOS (47)                                              Total: 2.3 GB    │
│                                                                            │
│  ▸ github.com/myorg/                                                       │
│    > api-service          A  98  main   2d ago    12.4 MB                  │
│    > frontend             B  81  feat   4h ago   890.2 MB  node_modules    │
│    > backend              C  62  main   3w ago     1.1 GB  target/ .venv   │
│    > old-project          F  18  main   8mo ago  340.0 MB  stale           │
│                                                                            │
│  [ENTER] Detail  [a] Add path  [c] Clean  [x] Delete  [s] Sort  [TAB] Next│
└────────────────────────────────────────────────────────────────────────────┘
```

## Install

| Method | Command |
|--------|---------|
| **npm** | `npm install -g @dpeluche/spark` |
| **curl** | `curl -fsSL https://raw.githubusercontent.com/dPeluChe/spark/main/scripts/install.sh \| sh` |
| **cargo** | `cargo install --git https://github.com/dPeluChe/spark` |
| **Binary** | [GitHub Releases](https://github.com/dPeluChe/spark/releases) — macOS arm64/x64, Linux x64 |

After installing:

```bash
spark init    # shell integration, completions, config
spark         # open the TUI
spark doctor  # validate your setup
```

## Modules

### Scanner — Repository Health Analyzer

Discovers git repos across your filesystem, scores health (A–F, 0–100), and cleans stale build artifacts.

- **Health grades**: last commit age, branch status, artifact sizes, ignored files
- **Artifact cleanup**: `node_modules`, `.venv`, `target/`, `.next`, `dist`, build caches (20+ types)
- **Container detection**: workspace folders (npm, pnpm, turborepo, nx, cargo, go)
- **Custom scan paths**: add any directory with `[a]`
- **Grouped results** by parent directory with sortable columns

### Repo Manager — ghq-style Repository Organizer

Clone, track, and manage all your repositories from one place, organized by `host/owner/name`.

```
~/repos/
├── github.com/
│   ├── myorg/api-service    main  2d ago
│   ├── myorg/frontend       feat  4h ago
│   └── oss/ripgrep          main  up to date
└── gitlab.com/
    └── company/internal     dev   behind ↓3
```

- Status tracking: ahead/behind/dirty, cached for 4 hours
- **Tagging**: group repos by project, client, or topic
- `spark pull all --tag work` — pull all repos in a group at once
- `spark status --tag ai-tools` — check status across a group

### Port Scanner — Dev Server Monitor

Find and kill development servers and processes running on your machine.

```bash
spark ps

  DEV SERVERS (3)
  PORT    PID      PROCESS    RUNTIME    PROJECT
  ------  -------  ---------  ---------  -----------------------
  3000    12345    node       Node.js    ~/code/frontend
  8080    23456    python3    Python     ~/code/api
  9090    34567    cargo      Rust       ~/code/service
```

- Detects Node.js, Python, Go, Rust, Ruby, Java, and more
- Separates dev servers from system services
- Kill by port, PID, or name — interactive or scripted (`spark ps node --kill`)

### System Cleanup — Docker, Caches, VMs, Logs

Clean disk space safely. Every item shows its risk level before anything is deleted.

- **Risk indicators**: safe (green) · caution (yellow) · danger (red)
- **Confirmation modal** with explanation per item, or bulk-clean selected with `[x]`
- **Docker**: dangling images, stopped containers, build cache
- **Caches**: Homebrew, npm, pip, Cargo, Xcode, CocoaPods, Go, Gradle
- **Logs**: dev logs >10 MB older than 7 days
- **VMs**: Docker VM disk, Android emulators, legacy VMs

Safety: path blocklist (`/System`, `/bin`, `/usr`...), app-aware checks, age filters, operation log, whitelist, dry-run mode. Inspired by [tw93/mole](https://github.com/tw93/mole).

### Security Audit — Secrets, OWASP, Dependencies

4-phase scanner for any project directory. Set the folder with `[a]` in the TUI or pass it as an argument.

1. **Secrets**: API keys (AWS, GitHub, Anthropic, OpenAI, Stripe, Slack), credentials, `.env` files
2. **Git history**: walks commit diffs for secrets committed and later removed
3. **OWASP Top 10:2025**: SQL injection, command injection, XSS, insecure crypto, path traversal, deserialization
4. **Dependencies**: [OSV.dev](https://osv.dev) batch API + `npm audit` for known CVEs

Context-aware severity (source code > config > test > docs). `.sparkauditignore` to suppress reviewed findings.

### Certificate Scanner — SSL/TLS Health Check

```bash
spark certs           # Keychain + home directory scan
spark certs --expired # Only expired certs
spark certs --summary # Counts by status
```

- macOS Keychain: expired, expiring soon, valid — grouped by issuer
- Home directory: loose `.pem`, `.crt`, `.key`, SSH keys
- Recommendations: Apple certs (safe), Developer (renew in Xcode), Self-signed (review and rotate)

### Updater — Dev Tool Manager

Tracks and updates 55 developer tools across 8 categories: AI tools, terminals, IDEs, infrastructure, runtimes, utilities, productivity, system.

Table view with current version, available version, and status. Update one tool or all outdated at once.

---

## CLI

```bash
spark                          # Open TUI
spark init                     # Shell integration, completions, config
spark doctor                   # Validate installation

# Repos
spark clone <url>              # Clone (ghq-compatible, owner/repo shorthand)
spark clone <url> -p           # Clone via SSH
spark list [-p] [query]        # List repos (tree: branch + age + tags)
spark search <query>           # Search repos
spark status [query]           # Check which repos need pull
spark status --tag <tag>       # Status filtered by tag
spark pull <query|all>         # Pull repos (ff-only)
spark pull all --tag <tag>     # Pull repos by tag
spark cd <name>                # Print path to repo
spark rm <query>               # Remove a repo

# Tags
spark tag add <repo> <tag>     # Tag a repo
spark tag remove <repo> <tag>  # Remove tag
spark tag list [tag]           # List tags or repos in a tag
spark tag delete <tag>         # Delete a tag
spark tag rename <old> <new>   # Rename a tag

# Ports
spark ps                       # Dev servers (pid, process, runtime, project)
spark ps --all                 # All ports: dev + macOS + services + apps
spark ps <query>               # Search processes by name
spark ps --kill <target>       # Kill by port, PID, or name
spark ps <query> --kill        # Non-interactive kill (exit 0/1 for scripts)

# Security audit
spark audit [path]             # Full audit (secrets + OWASP + deps)
spark audit --deps             # Dependency-only scan
spark audit --offline          # No network
spark audit --init             # Create .sparkauditignore
spark audit -o report.txt      # Save report to file

# Certificates
spark certs                    # Keychain + home directory
spark certs --keychain         # Keychain only
spark certs --expired          # Only expired
spark certs --summary          # Counts only

# Config
spark root [--set <path>]      # Show/change repos root
spark config [key --set v]     # Show/update config
spark completions <shell>      # zsh/bash/fish completions
spark agent                    # AI agent integration tips
spark --dry-run                # Preview mode (no destructive actions)
```

---

## Keyboard Controls

| Tab | Key | Action |
|-----|-----|--------|
| **Global** | `TAB` | Cycle tabs: Scanner → Repos → Ports → System → Audit → Updater |
| | `q` | Back / close modal |
| | `Ctrl+C` | Quit |
| **Scanner** | `ENTER` | Scan directory / view repo detail |
| | `a` | Add custom scan path |
| | `c` | Clean artifacts |
| | `x` | Delete repo |
| | `s` | Sort results |
| | `?` | Health grade explanation |
| **Repos** | `ENTER` | Action modal (pull, open, delete) |
| | `c` | Clone a repo |
| | `u` / `U` | Pull selected / pull all behind |
| | `r` | Refresh statuses |
| **Ports** | `ENTER` | Action modal (kill, open folder) |
| | `SPACE` | Select |
| | `x` / `X` | Kill selected / kill all dev servers |
| **System** | `ENTER` | Detail/risk modal |
| | `SPACE` | Select item |
| | `x` | Clean selected |
| **Audit** | `a` | Set folder to scan |
| | `ENTER` | View findings detail |
| | `r` | Rescan |
| **Updater** | `u` / `U` | Update selected / update all |
| | `ENTER` | View changelog |

---

## Configuration

```bash
spark config                          # Show all settings
spark config repos_root --set ~/code  # Change repos root
spark root --set ~/code               # Same thing
```

Config file: `~/.config/spark/config.toml` (macOS: `~/Library/Application Support/spark/`)

| File | Purpose |
|------|---------|
| `config.toml` | Main configuration |
| `whitelist.txt` | Paths to protect during system cleanup |
| `operations.log` | Audit log of cleanup actions |
| `repo_status_cache.json` | Cached repo statuses (4hr TTL) |

```toml
# ~/.config/spark/config.toml
repos_root = "~/repos"
stale_threshold_days = 90
large_artifact_threshold = 104857600  # 100 MB
use_trash = true
max_scan_depth = 6
```

---

## For AI Agents

```bash
spark agent    # Integration tips
spark ingest   # Generate LLM-ready context digest (via trs)
```

Add to your `CLAUDE.md` or `.cursorrules`:

```
Repos managed by spark (ghq-compatible).
Run `spark cd <name>` to find repo paths.
Run `spark root` to get the repos root.
Run `spark list` for a full repo tree.
```

Shell navigation:
```bash
spark init      # Adds spark-cd function to your shell
spark-cd zed    # cd to the zed repo
```

---

## Tech Stack

| | |
|---|---|
| Language | Rust |
| TUI | [Ratatui](https://ratatui.rs) + [crossterm](https://github.com/crossterm-rs/crossterm) |
| Async | [tokio](https://tokio.rs) |
| Git | [git2](https://github.com/rust-lang/git2-rs) (libgit2 bindings) |
| HTTP | [reqwest](https://github.com/seanmonstar/reqwest) + rustls |
| CLI | [clap 4](https://github.com/clap-rs/clap) |
| Binary | ~4 MB (LTO + strip), no runtime deps |
| Tests | 127 passing, 0 warnings |

---

## Contributing

```bash
git clone https://github.com/dPeluChe/spark.git
cd spark
cargo test                  # all tests must pass
cargo clippy -- -D warnings # no warnings allowed
cargo fmt -- --check        # formatting must match
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines, [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the codebase map, and [docs/TASK_TODO.md](docs/TASK_TODO.md) for the roadmap.

---

## Acknowledgments

Built with [Ratatui](https://ratatui.rs), [tokio](https://tokio.rs), [git2](https://github.com/rust-lang/git2-rs), [clap](https://github.com/clap-rs/clap).

Inspired by [ghq](https://github.com/x-motemen/ghq), [mole](https://github.com/tw93/mole), [lazygit](https://github.com/jesseduffield/lazygit), [k9s](https://k9scli.io).

---

**SPARK** v0.5.1 — MIT License
