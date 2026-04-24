# SPARK — Task TODO

Pending tasks and improvements for the SPARK DevOps platform.

> Completed tasks archived in [TASK_COMPLETED/](./TASK_COMPLETED/) by month.

---

## High Priority

### Updater: runtime version manager sub-panel `added: 2026-04-20`
- Show installed versions per runtime (nvm ls, pyenv versions, rvm list, rustup toolchain list)
- Accessible via Enter on a runtime tool in the updater table
- Display as detail panel similar to repo detail in Scanner

---

## Medium Priority

### Workspace sub-project listing `added: 2026-04-20`
- Inside repo detail, show workspace sub-projects (npm workspaces, cargo workspace members)
- Parse `package.json` workspaces, `Cargo.toml` [workspace] members, `pnpm-workspace.yaml`

### Persist scan paths in config `added: 2026-04-20`
- Save user's selected scan directories in `config.toml`
- Auto-load on next TUI launch (skip directory selection if paths saved)
- Add `spark config --scan-dirs` to manage from CLI

### Audit: git history false positives `added: 2026-04-20`
- History scanner detects test fixtures from scanner's own test code in commit diffs
- Could parse surrounding diff context to detect test blocks

### Audit: parallel scan phases `added: 2026-04-20`
- Phases 1-3 (secrets, history, patterns) run sequentially
- Could parallelize since they don't share state

---

## Low Priority

### Docker image testing `added: 2026-04-20`
- Create Dockerfile for testing spark on clean Linux (Ubuntu, Alpine)
- Validate install.sh works in containerized environments
- Test cargo install path on fresh Linux

### Parallel status fetching `added: 2026-04-20`
- `spark status` fetches repos sequentially (slow for 50+ repos)
- Consider parallel fetch with concurrency limit (e.g., 5 at a time)
- Show progress bar instead of repo-by-repo counter

### TUI repo detail for non-containers `added: 2026-04-20`
- Pressing Enter on a non-container repo in ScanResults goes to RepoDetail
- Could show richer info: recent commits, branch list, disk usage breakdown

### Audit: AST-based parsing for code patterns `added: 2026-04-15`
- Today `scanner/code_patterns/` uses pure regex for OWASP Top 10 detection
- Replace with layered strategy: AST parsing first (tree-sitter), regex fallback for unsupported langs
- Benefit: fewer false positives (e.g. detections inside string literals or comments)
- Inspiration: CodeFlow (github.com/braedonsaunders/codeflow) uses Acorn + Tree-Sitter WASM with regex fallback for 40+ languages
- Tree-sitter is reachable indirectly via `trs -l aggressive` (compression), not a spark dep — AST parsing would add tree-sitter as a direct dep
- Start with JS/TS and Python (highest false-positive risk), keep regex for Rust/Go

### Audit: more ecosystems `added: 2026-04-20`
- Support `go.sum` (Go modules), `Gemfile.lock` (Ruby), `composer.lock` (PHP)
- Support `pnpm-lock.yaml`, `yarn.lock` for npm alternatives
