# SPARK v0.8.0

**SPARK** is a developer operations platform built as a high-performance TUI. Manage repos, scan health, clean artifacts, monitor ports, update dev tools — all from one place.

```
   _____ ____  ___  ____  __ __
  / ___// __ \/   |/ __ \/ //_/
  \__ \/ /_/ / /| / /_/ / ,<
 ___/ / ____/ ___ / _, _/ /| |
/____/_/   /_/  |/_/ |_/_/ |_|

   Developer Operations Platform v0.8.0
```

Built with **Rust**, **Ratatui**, and **tokio**.

---

## Install

```bash
# npm (recommended)
npm install -g @dpeluche/spark

# npx (run without installing)
npx @dpeluche/spark

# cargo
cargo install --git https://github.com/dPeluChe/labs-spark

# from source
git clone https://github.com/dPeluChe/labs-spark && cd labs-spark
./install.sh
```

After installing:
```bash
spark init    # Setup shell integration, completions, whitelist
spark         # Open the TUI
```

---

## Modules

### Scanner — Repository Health Analyzer

Discovers git repos across your system, scores their health, and cleans stale artifacts.

- Health scoring (A-F grades, 0-100 points)
- Container detection (folders with repos inside)
- Workspace detection (npm, pnpm, turborepo, nx, cargo, go)
- Artifact cleanup: `node_modules`, `.venv`, `target/`, build caches
- Grouped results by parent directory

### Repo Manager — ghq-compatible Repository Organizer

Clone, track, and maintain all your repositories from one place.

```
~/repos/github.com/user/my-api/
~/repos/github.com/user/frontend/
~/repos/gitlab.com/company/internal-tool/
```

- Clone with auto-organization by host/owner/name
- Status tracking: ahead/behind/dirty (cached for 4 hours)
- Pull all behind repos at once
- Size column, last commit, branch info

### Port Scanner — Dev Server Monitor

Find and manage development servers running on your machine.

- Detects Node.js, Python, Go, Rust, Ruby, and more
- Groups dev servers vs system services
- Project path detection via working directory
- Kill processes from the TUI

### System Cleanup — Docker, Caches, VMs, Logs

Clean up disk space with safety guards inspired by [tw93/mole](https://github.com/tw93/mole).

- **Docker**: dangling images, stopped containers, build cache
- **VMs**: Docker VM disk, Android emulators, legacy VMs
- **Caches**: Homebrew, npm, pip, Cargo, Xcode, CocoaPods, Go, Gradle
- **Logs**: dev logs >10MB older than 7 days
- **Downloads**: ISOs, DMGs, PKGs >50MB

**Safety**: path validation, app-aware (skips running apps), age-based filtering, operation logging, whitelist support, dry-run mode.

### Security Audit — Secrets, Code Patterns, Dependencies

4-phase security scanner for any project:

1. **Secrets scan**: API keys (AWS, GitHub, Anthropic, OpenAI, Stripe, Slack), credentials, sensitive files, .env
2. **Git history**: Scans commit diffs for secrets that were committed and later removed
3. **Code patterns (OWASP Top 10:2025)**: SQL injection, command injection, XSS, insecure crypto, deserialization, path traversal
4. **Dependencies**: Queries [OSV.dev](https://osv.dev) API + `npm audit` for known vulnerabilities

Context-aware severity (source code > config > test > docs). Supports `.sparkauditignore` for suppressing reviewed findings.

### Certificate Scanner — SSL/TLS Health Check

Scan and audit certificates across your system:

- **Keychain scan** (macOS): expired, expiring, valid — grouped by issuer
- **Home directory scan**: finds loose `.pem`, `.key`, `.crt`, SSH keys across `~/`
- **Recommendations by type**: Apple (safe to remove), Developer (renew), Self-signed (review and rotate)
- **Expiration analysis**: summary by age, cert file status parsing

### System Cleanup — Docker, Caches, VMs, Logs

Clean up disk space with safety guards inspired by [tw93/mole](https://github.com/tw93/mole).

- Risk indicators per item: **safe** (green), **caution** (yellow), **danger** (red)
- Confirmation popup with explanation before cleaning any item
- **Docker**: dangling images, stopped containers, build cache
- **Caches**: Homebrew, npm, pip, Cargo, Xcode, CocoaPods, Go, Gradle
- **Logs**: dev logs >10MB older than 7 days
- **VMs**: Docker VM disk, Android emulators, legacy VMs
- **Downloads**: ISOs, DMGs, PKGs >50MB

### Updater — Tool Update Manager

Manages updates for 55 developer tools across 8 categories: AI tools, terminals, IDEs, productivity, infrastructure, utilities, runtimes, system. Table view with version comparison and status indicators.

---

## CLI Commands

```bash
spark                      # Open TUI
spark init                 # Setup shell integration + completions
spark clone <url>          # Clone repo (ghq-compatible, owner/repo shorthand)
spark clone <url> -p       # Clone via SSH
spark clone <url> --shallow # Shallow clone
spark cd <name>            # Print path to repo
spark search <query>       # Search repos (shows status, commit age, path)
spark list [-p] [query]    # List repos (tree by host/owner)
spark status [query]       # Check which repos need pull (fetch + compare)
spark pull <query|all>     # Pull repos behind remote (ff-only)
spark audit [path]         # Security audit (secrets + OWASP + deps)
spark audit --deps         # Dependency-only scan (OSV.dev + npm audit)
spark audit --offline      # Local-only scan (no network)
spark audit --init         # Create .sparkauditignore
spark audit -o report.txt  # Save report to file
spark certs                # Scan SSL/TLS certs (Keychain + files + ~/)
spark certs --expired      # Show only expired
spark certs --summary      # Summary only
spark root [--set <path>]  # Show/change repos root
spark rm <query>           # Remove a repo
spark doctor               # Validate installation + environment
spark config [key --set v] # Show/update configuration
spark agent                # AI agent integration tips
spark completions <shell>  # Generate zsh/bash/fish completions
spark --dry-run            # TUI in preview mode (no destructive actions)
```

---

## Keyboard Controls

### Global
| Key | Action |
|-----|--------|
| `TAB` | Cycle: Scanner → Repos → Ports → System → Audit → Updater |
| `q` | Back / Close modal |
| `Ctrl+C` | Quit |

### Scanner
| Key | Action |
|-----|--------|
| `ENTER` | Scan selected directory / View repo detail |
| `SPACE` | Toggle selection |
| `a` | Add custom scan path |
| `d` | Remove scan path |
| `r` | Refresh directories |
| `c` | Clean artifacts |
| `x` | Delete repo |
| `s` | Sort results |
| `?` | Health grade explanation |
| `Home/End` | Jump to start/end |
| `PgUp/PgDn` | Page navigation |

### Repos
| Key | Action |
|-----|--------|
| `ENTER` | Open action modal (pull, open, delete) |
| `c` | Clone a repo |
| `u` | Pull selected repos |
| `U` | Pull all behind repos |
| `r` | Refresh (re-fetch all statuses) |

### Ports
| Key | Action |
|-----|--------|
| `ENTER` | Open action modal (kill, open folder) |
| `SPACE` | Select ports |
| `x` | Kill selected |
| `X` | Kill all dev servers |
| `r` | Rescan |

### System Cleanup
| Key | Action |
|-----|--------|
| `ENTER` | Clean selected item |
| `SPACE` | Toggle selection |
| `x` | Clean selected items |
| `r` | Rescan |

### Audit
| Key | Action |
|-----|--------|
| `ENTER` | View project findings detail |
| `j/k` | Navigate projects / findings |
| `r` | Rescan |
| `PgUp/PgDn` | Page navigation |

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
| `whitelist.txt` | Paths to skip during system cleanup |
| `operations.log` | Audit log of cleanup operations |
| `repo_status_cache.json` | Cached repo statuses (4hr expiry) |

---

## For AI Agents

```bash
spark agent    # Show integration tips
```

Add to your CLAUDE.md or .cursorrules:
```
Repos managed by spark. Run `spark cd <name>` to find repo paths.
Repos root: run `spark root`
```

Shell integration:
```bash
spark init    # Adds spark-cd to your shell
spark-cd zed  # Navigate to zed repo
```

---

## Safety

System cleanup follows [tw93/mole](https://github.com/tw93/mole) safety principles:

- **Path validation**: blocks `/`, `/System`, `/bin`, `/usr`, `/etc`, `/Library`
- **App-aware**: checks if app is running before cleaning its cache
- **Age-based**: only cleans logs older than 7 days
- **Operation log**: every action logged to `operations.log`
- **Whitelist**: user-editable `whitelist.txt` to protect paths
- **Dry-run**: `spark --dry-run` previews without deleting

---

## Development

```bash
cargo run                  # Run in dev mode
cargo test                 # 79 tests
cargo build --release      # Optimized build
```

---

## Acknowledgments

Built with [Ratatui](https://ratatui.rs), [tokio](https://tokio.rs), [git2](https://github.com/rust-lang/git2-rs), [crossterm](https://github.com/crossterm-rs/crossterm).

Inspired by [ghq](https://github.com/x-motemen/ghq), [mole](https://github.com/tw93/mole), [lazygit](https://github.com/jesseduffield/lazygit), [k9s](https://k9scli.io).

---

**SPARK** v0.8.0 — Developer Operations Platform
