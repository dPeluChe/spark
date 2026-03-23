# SPARK v0.8.0

**SPARK** is a developer operations platform built as a high-performance TUI. What started as a simple update script evolved through four generations into a comprehensive tool for managing your entire dev environment.

```
   _____ ____  ___  ____  __ __
  / ___// __ \/   |/ __ \/ //_/
  \__ \/ /_/ / /| / /_/ / ,<
 ___/ / ____/ ___ / _, _/ /| |
/____/_/   /_/  |/_/ |_/_/ |_|

   Developer Operations Platform v0.8.0
```

Built with **Rust**, **Ratatui**, and **tokio**. ~5,500 lines of Rust.

---

## The Evolution

SPARK has gone through a revolution at each stage, evolving from a bash script into a full DevOps platform:

```
v0.4.x  Bash Script       A collection of shell functions. Manual, fragile.
          |
v0.5.x  Go + Bubble Tea   First TUI. Grid layout, splash screen, categories.
          |
v0.6.0  Go (Mature)       71 tools, search, dry-run, danger zone modals.
          |                ~1,956 lines of Go.
          |
v0.7.0  Rust Migration    Complete rewrite. Rust + Ratatui + tokio.
          |                Added Scanner: repo health, artifact cleanup.
          |                Cross-platform. Async everything.
          |
v0.8.0  DevOps Platform   Repo Manager (ghq-style clone/pull/status).
                           Port Scanner (find/kill dev servers).
                           Post-clone summary with agent integration tips.
                           Configurable repos_root. ~5,500 lines of Rust.
```

Each version was a leap, not an increment. The Rust migration wasn't a port -- it was a rethink. And v0.8.0 transforms SPARK from "update tool" into "dev environment manager."

---

## Core Modules

### 1. Updater -- Tool Update Manager

Manages updates for 44+ developer tools across 8 categories.

| Category | Tools |
|----------|-------|
| **AI Development** | Claude, Droid, Gemini, OpenCode, Codex, Crush, Toad |
| **Terminals** | iTerm2, Ghostty, Warp |
| **IDEs & Editors** | VS Code, Cursor, Zed, Windsurf, Antigravity |
| **Productivity** | JQ, FZF, Ripgrep, Bat, HTTPie, LazyGit, TLDR |
| **Infrastructure** | Docker, Kubernetes, Helm, Terraform, AWS CLI, Ngrok |
| **Utilities** | Git, Tmux, Zellij, Oh My Zsh, SQLite, Watchman, Direnv |
| **Runtimes** | Node.js, Python, Go, Ruby, PostgreSQL |
| **System** | Homebrew Core, NPM Globals |

- Parallel version detection via tokio
- Danger zone modals for critical runtimes
- Dry-run preview before executing
- Search & filter across all tools

### 2. Scanner -- Repository Health Analyzer

Discovers git repos across your system and scores their health.

- Health scoring (0-100, grades A-F)
- Stale repo detection (configurable threshold)
- Artifact discovery: `node_modules`, `.venv`, `target/`, build caches
- Cleanup actions: trash, archive, or delete artifacts
- Size analysis with human-readable breakdown

### 3. Repo Manager -- ghq-style Repository Organizer

Clone, track, and maintain all your repositories from one place.

```
~/repos/                          # Configurable via repos_root
  github.com/
    user/my-api/
    user/frontend/
  gitlab.com/
    company/internal-tool/
```

- **Clone**: `[c]` -- paste any git URL, auto-organized by host/owner/name
- **Status**: See branch, ahead/behind, dirty state at a glance
- **Pull**: `[u]` selected or `[U]` all behind repos
- **Post-clone summary**: After cloning, shows:
  - Full path and alias suggestion (`alias my_api='cd ~/repos/...'`)
  - Instructions for AI agents (CLAUDE.md, .cursorrules paths)
  - Quick access commands

### 4. Port Scanner -- Dev Server Monitor

Find and manage development servers running on your machine.

- Detects common dev ports (3000, 5173, 8080, etc.)
- Shows PID, runtime (Node, Python, Go, Ruby, Rust), and command
- Kill processes directly from the TUI

---

## Quick Start

```bash
# Clone and build
git clone <repo-url> && cd labs-spark
cargo build --release

# Install
cp target/release/spark ~/.local/bin/spark

# Run
spark
```

---

## Keyboard Controls

### Global
| Key | Action |
|-----|--------|
| `TAB` | Switch between Updater / Scanner tabs |
| `Q` or `Ctrl+C` | Quit |

### Updater
| Key | Action |
|-----|--------|
| `j/k` or `Up/Down` | Navigate tools |
| `SPACE` | Toggle selection |
| `G` / `A` | Select category / all |
| `/` | Search & filter |
| `D` | Dry-run preview |
| `ENTER` | Start updates |

### Scanner
| Key | Action |
|-----|--------|
| `G` | Open Repo Manager |
| `P` | Open Port Scanner |
| `ENTER` | Start scan |
| `SPACE` | Select repos for cleanup |

### Repo Manager
| Key | Action |
|-----|--------|
| `c` | Clone a repo (enter URL) |
| `SPACE` | Select repos |
| `u` | Pull selected repos |
| `U` | Pull all behind repos |
| `r` | Refresh status |

---

## Configuration

SPARK reads from `~/.config/spark/config.toml`:

```toml
# Directories to scan for git repos
scan_directories = ["~/Projects", "~/Developer", "~/Code", "~/repos"]

# Stale threshold (days since last commit)
stale_threshold_days = 90

# Minimum artifact size to flag (bytes, default 100MB)
large_artifact_threshold = 104857600

# Use OS trash for safe deletions
use_trash = true

# Max scan depth
max_scan_depth = 4

# Root for managed repos (ghq-style layout)
repos_root = "~/repos"
```

See [config.example.toml](config.example.toml) for all options.

---

## Architecture

```
src/
├── main.rs                     # Entry point, terminal setup, tokio runtime
├── app.rs                      # Event loop, action dispatch via mpsc channels
├── config.rs                   # SparkConfig from ~/.config/spark/config.toml
├── core/
│   ├── types.rs                # Tool, Category, UpdateMethod, ToolStatus
│   ├── inventory.rs            # 44+ tools catalog
│   └── changelogs.rs           # Changelog URL mappings
├── updater/
│   ├── detector.rs             # Version detection (brew, npm, CLI, macOS apps)
│   ├── version.rs              # Regex-based version parsing
│   └── executor.rs             # Update execution
├── scanner/
│   ├── repo_scanner.rs         # Git repo discovery + analysis via git2
│   ├── space_analyzer.rs       # Artifact detection
│   ├── health.rs               # Health scoring (A-F grades)
│   ├── cleaner.rs              # Cleanup: trash, archive, delete
│   ├── repo_manager.rs         # ghq-style clone, pull, status tracking
│   └── port_scanner.rs         # Dev server port discovery + kill
├── tui/
│   ├── model.rs                # All state: App, Updater, Scanner, RepoManager, Ports
│   ├── update.rs               # Message handling, state transitions
│   ├── scanner_keys.rs         # Scanner-specific key bindings
│   ├── view.rs                 # Top-level render + tab bar
│   ├── styles.rs               # Colors, ASCII art, spinners
│   └── widgets/
│       ├── splash.rs           # Animated splash with color cycling
│       ├── dashboard.rs        # Tool grid (2 columns, 8 categories)
│       ├── scanner_view.rs     # Scan results, config, cleaning views
│       ├── detail_panel.rs     # Single repo detail + artifacts
│       ├── repo_manager_view.rs # Repo list, clone input, clone summary
│       ├── port_view.rs        # Port scanner table + actions
│       ├── progress.rs         # Progress bars + overlays
│       ├── modal.rs            # Confirmation modals
│       └── search.rs           # Inline search bar
└── utils/
    ├── shell.rs                # Async shell with timeouts
    └── fs.rs                   # Dir size, git root discovery
```

### Key Design Decisions

- **tokio async runtime**: All I/O (version checks, git ops, filesystem scans) runs concurrently
- **mpsc channels**: Background tasks send `AppMessage` back to the TUI event loop
- **Action dispatch**: Key handlers return `Action` enums, app.rs executes side effects
- **State machines**: Each module has explicit states with validated transitions

---

## For AI Agents

When an AI agent (Claude Code, Cursor, Codex) needs to work with a repo managed by SPARK:

1. The post-clone summary provides the exact path after `[c]` clone
2. Add to your agent config:
   - **CLAUDE.md**: `Repo path: ~/repos/github.com/owner/name`
   - **.cursorrules**: `Project root: ~/repos/github.com/owner/name`
3. Or create a shell alias: `alias myrepo='cd ~/repos/github.com/owner/name'`

The ghq-style layout means paths are predictable: `{repos_root}/{host}/{owner}/{name}`

---

## Documentation

| Document | Description |
|----------|-------------|
| [INSTALLATION.md](docs/INSTALLATION.md) | Setup and troubleshooting |
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | Technical deep-dive |
| [WORKFLOWS.md](docs/WORKFLOWS.md) | User interaction flows |
| [ADDING_TOOLS.md](docs/ADDING_TOOLS.md) | How to add new tools |
| [CLAUDE.md](CLAUDE.md) | Developer guidance for Claude Code |
| [config.example.toml](config.example.toml) | All configuration options |

---

## Version History

| Version | Era | What Changed |
|---------|-----|-------------|
| **v0.8.0** | **DevOps Platform** | Repo Manager (ghq clone/pull/status), Port Scanner, post-clone summary with agent tips, configurable `repos_root` |
| **v0.7.0** | **Rust Migration** | Complete rewrite: Rust + Ratatui + tokio. Repository Scanner with health scoring, artifact cleanup. Cross-platform |
| **v0.6.0** | **Go Mature** | 71 tools, search, dry-run preview, danger zone, state machine |
| **v0.5.x** | **Go TUI** | First TUI with grid layout, splash screen, categories |
| **v0.4.x** | **Bash Script** | Original shell scripts (archived) |

---

## Development

```bash
# Run in dev mode
cargo run

# Build optimized release
cargo build --release

# Add a new tool: edit src/core/inventory.rs
```

See [docs/ADDING_TOOLS.md](docs/ADDING_TOOLS.md) for the full guide.

---

## Acknowledgments

Built with:
- [Ratatui](https://ratatui.rs) -- Rust TUI framework
- [tokio](https://tokio.rs) -- Async runtime
- [git2](https://github.com/rust-lang/git2-rs) -- Git operations
- [crossterm](https://github.com/crossterm-rs/crossterm) -- Terminal manipulation

Inspired by [ghq](https://github.com/x-motemen/ghq), [lazygit](https://github.com/jesseduffield/lazygit), [k9s](https://k9scli.io).

---

**SPARK** v0.8.0 -- Developer Operations Platform
*From bash script to DevOps platform. Four generations of evolution.*
