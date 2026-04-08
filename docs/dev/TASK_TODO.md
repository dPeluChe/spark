# SPARK — Task TODO

Pending tasks and improvements for the SPARK DevOps platform.

---

## High Priority

### AI CLI integration — Skills for Claude Code, Codex, Gemini, Droid
- Create a spark skill (.md file) that teaches AI agents when/how to use spark CLI
- Skill covers: spark audit (security), spark status/pull (repos), spark certs, spark doctor
- `spark init` auto-installs the skill in ~/.claude/skills/, ~/.codex/, ~/.gemini/
- Each AI agent invokes spark via Bash (no MCP needed — spark is a standard CLI)
- Skill should be context-aware: suggest audit before commits, status before deployments
- **Why**: Make spark discoverable and usable by AI coding agents automatically

### Updater: runtime version manager sub-panel
- Show installed versions per runtime (nvm ls, pyenv versions, rvm list, rustup toolchain list)
- Accessible via Enter on a runtime tool in the updater table
- Display as detail panel similar to repo detail in Scanner

### `spark smoke-test` / CI validation script
- Create `scripts/smoke-test.sh` for portable post-install validation
- Test: `spark --version`, `spark --help`, `spark list`, `spark config`
- Test: `spark clone` + `spark cd` + `spark rm` round-trip
- Test: `spark status` and `spark pull` against a known public repo
- Run in GitHub Actions CI after each release build
- **Why**: Ensure cross-platform releases (macOS arm64/x64, Linux) actually work

### Audit: TUI integration completion
- Audit tab shows project list + detail but doesn't show dependency findings in TUI yet
- Add dependency scan results to TUI audit detail view
- Show npm audit results in TUI

---

## Medium Priority

### System cleaner navigation
- Cursor jumps between groups (flat item index vs grouped display with headers)
- Same issue that was fixed in Scanner — needs equivalent group-aware scroll

### Workspace sub-project listing
- Inside repo detail, show workspace sub-projects (npm workspaces, cargo workspace members)
- Parse `package.json` workspaces, `Cargo.toml` [workspace] members, `pnpm-workspace.yaml`

### Persist scan paths in config
- Save user's selected scan directories in `config.toml`
- Auto-load on next TUI launch (skip directory selection if paths saved)
- Add `spark config --scan-dirs` to manage from CLI

### ESC key support
- ESC doesn't work in some terminals (only `q` works for back/close)
- Investigate terminal-specific ESC handling (Ghostty, Zellij, iTerm2)

### Audit: git history false positives
- History scanner detects test fixtures from scanner's own test code in commit diffs
- Could parse surrounding diff context to detect test blocks

### Audit: parallel scan phases
- Phases 1-3 (secrets, history, patterns) run sequentially
- Could parallelize since they don't share state

---

## Low Priority

### Docker image testing
- Create Dockerfile for testing spark on clean Linux (Ubuntu, Alpine)
- Validate install.sh works in containerized environments
- Test cargo install path on fresh Linux

### Parallel status fetching
- `spark status` fetches repos sequentially (slow for 50+ repos)
- Consider parallel fetch with concurrency limit (e.g., 5 at a time)
- Show progress bar instead of repo-by-repo counter

### TUI repo detail for non-containers
- Pressing Enter on a non-container repo in ScanResults goes to RepoDetail
- Could show richer info: recent commits, branch list, disk usage breakdown

### Audit: more ecosystems
- Support `go.sum` (Go modules), `Gemfile.lock` (Ruby), `composer.lock` (PHP)
- Support `pnpm-lock.yaml`, `yarn.lock` for npm alternatives
