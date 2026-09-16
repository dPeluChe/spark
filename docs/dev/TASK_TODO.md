# SPARK — Task TODO

Pending tasks and improvements for the SPARK DevOps platform.

> Completed tasks archived in [TASK_COMPLETED/](./TASK_COMPLETED/) by month.
> Strategy and phase specs: [ROADMAP.md](./ROADMAP.md) — agent-first local DevOps.

---

## Current focus — Agent-First Toolkit (see ROADMAP.md)

### Phase 0: machine-readable outputs `added: 2026-09-16`
- `--json` for `status`, `list`, `ps`, `audit` (schemas in ROADMAP.md, `json_version: 1`)
- `audit` exit 1 on findings (CI gate); `status --exit-code` opt-in for agents/CI
- `RepoStatus::Dirty` carries `ahead`/`behind` (today dirty masks behind/diverged)
- Add `Serialize` derives to scanner finding structs

### Phase 1: `spark report` (fleet view) `added: 2026-09-16`
- One command: repos summary + disk (artifacts/system) + ports + tools + last audit
- Human output + `--json`; fast by default (caches), `--fresh` re-fetches
- Parallelize per-repo artifact scan with the 8-worker pool
- Acceptance: cold < 60s / warm < 5s on a 145-repo fleet

### Phase 2: agent loop closure `added: 2026-09-16`
- `spark.skill.md`: prefer `--json`, `report` for triage, `audit` exit codes for gates
- `spark audit` persists `last_audit.json` summary (feeds `spark report`)
- README/docs: document the agent workflow

---

## Backlog — features

### Updater: runtime version manager sub-panel `added: 2026-04-20`
- Show installed versions per runtime (nvm ls, pyenv versions, rvm list, rustup toolchain list)
- Accessible via Enter on a runtime tool in the updater table
- Display as detail panel similar to repo detail in Scanner

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

### TUI repo detail for non-containers `added: 2026-04-20`
- Pressing Enter on a non-container repo in ScanResults goes to RepoDetail
- Could show richer info: recent commits, branch list, disk usage breakdown

### Docker image testing `added: 2026-04-20`
- Create Dockerfile for testing spark on clean Linux (Ubuntu, Alpine)
- Validate install.sh works in containerized environments
- Test cargo install path on fresh Linux

---

## Parked (revisit criteria in ROADMAP.md)

### MCP server `parked: 2026-09-16`
- CLI + skill already covers in-house agents with fewer moving parts
- Revisit on third-party distribution or external adoption signal

### Cross-platform: linux-arm64, Windows `parked: 2026-09-16`
- Port scanner and cert scanner are macOS/Linux; fleet is macOS today
- Revisit on external adoption signal
