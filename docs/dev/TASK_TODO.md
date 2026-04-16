# SPARK — Task TODO

Pending tasks and improvements for the SPARK DevOps platform.

> Completed tasks archived in [TASK_COMPLETED/](./TASK_COMPLETED/) by month.

---

## High Priority

### spark ingest: switch to trs --output flag `added: 2026-04-16`
- When new trs version ships with `-o <path>` support, update `src/scanner/repo_ingest.rs`
- Remove stdout capture, replace with `args.push("-o"); args.push(output_path)`
- Read stdout to confirm path returned by trs
- One-liner change — see TODO comment in `generate_ingest()`

### Updater: runtime version manager sub-panel
- Show installed versions per runtime (nvm ls, pyenv versions, rvm list, rustup toolchain list)
- Accessible via Enter on a runtime tool in the updater table
- Display as detail panel similar to repo detail in Scanner

---

## Medium Priority

### Workspace sub-project listing
- Inside repo detail, show workspace sub-projects (npm workspaces, cargo workspace members)
- Parse `package.json` workspaces, `Cargo.toml` [workspace] members, `pnpm-workspace.yaml`

### Persist scan paths in config
- Save user's selected scan directories in `config.toml`
- Auto-load on next TUI launch (skip directory selection if paths saved)
- Add `spark config --scan-dirs` to manage from CLI

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

### Audit: AST-based parsing for code_patterns.rs `added: 2026-04-15`
- Today `code_patterns.rs` uses pure regex for OWASP Top 10 detection
- Replace with layered strategy: AST parsing first (tree-sitter), regex fallback for unsupported langs
- Benefit: fewer false positives (e.g. detections inside string literals or comments)
- Inspiration: CodeFlow (github.com/braedonsaunders/codeflow) uses Acorn + Tree-Sitter WASM with regex fallback for 40+ languages
- Tree-sitter already used in `spark ingest --compress` — leverage same crate
- Start with JS/TS and Python (highest false-positive risk), keep regex for Rust/Go

### Audit: more ecosystems
- Support `go.sum` (Go modules), `Gemfile.lock` (Ruby), `composer.lock` (PHP)
- Support `pnpm-lock.yaml`, `yarn.lock` for npm alternatives
