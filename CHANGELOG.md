# Changelog

All notable changes to SPARK are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- **`spark report`** (alias `spark overview`) — fleet status in one screen:
  repos by state, recoverable disk (repo artifacts + system cleanables), dev
  servers, outdated tools, and the last audit summary. Fast by default
  (warm ~1.5s on a 145-repo fleet); `--fresh` re-fetches statuses and tool
  versions; `--json` mirrors every section (`json_version: 1`). `spark audit`
  now persists a summary to `last_audit.json` to feed it.
- `--json` machine-readable output (`json_version: 1`) for `spark status`,
  `spark list`, `spark ps`, and `spark audit` — stable contracts for agents
  and CI, shapes documented in
  [`docs/dev/ROADMAP.md`](docs/dev/ROADMAP.md).
- `spark status --exit-code` — exits 1 when any repo needs attention.
- `spark audit` exits 1 when findings exist (CI gate).
- [`docs/dev/ROADMAP.md`](docs/dev/ROADMAP.md) — agent-first focus, phases,
  and the `--json` schema reference.
- `RepoStatus::Dirty` now carries `ahead`/`behind` — dirty repos report
  their real drift ("Dirty, 304 behind") instead of masking it.
- [`docs/dev/TRS_INTEGRATION.md`](docs/dev/TRS_INTEGRATION.md) — division of
  responsibilities between SPARK (fleet layer) and TRS (digest generator + storage)
  with lessons learned from the overlap.
- `scripts/version.sh` — atomic bump + consistency check across the 5
  version fields (Cargo.toml + 4 npm manifests). Invoked by release CI.
- CI gate (`verify-versions` job) that blocks the release if the git tag
  disagrees with Cargo.toml or if any manifest drifts.
- `CHANGELOG.md` (this file).

### Changed
- `spark status` summary now lists the exact `spark pull owner/name` command
  for every behind repo (copy-paste ready) and counts the repos that need
  manual action (dirty → commit, diverged → merge/rebase).
- `spark report` shows the running SPARK version in the header and as
  `spark_version` in the `--json` payload.
- **`spark.skill.md` v1.1.0**: agents start with `spark report --json` for
  fleet triage, prefer the JSON contracts over parsing prose, and use the
  `audit` / `status --exit-code` exit codes as gates.
- README / README.es: agent workflow section rewritten around `--json` and
  `spark report`; CLI references completed. ARCHITECTURE, CLAUDE.md, and
  WORKFLOWS refreshed for `report.rs` / `json.rs`.
- **`spark status` / `spark pull` fetch repos in parallel** (bounded pool of
  8 workers). A 145-repo fleet checks fresh in ~21s and pulls in ~27s
  (previously sequential: minutes).
- `spark pull` merges via `merge --ff-only @{upstream}` on the refs the
  status check already fetched — no second network fetch per repo.
- `spark pull` output is bucketed: pulled / up to date / skipped (dirty,
  ahead, diverged) / errors, each error condensed to one line with a
  remediation hint (`spark rm` for dead remotes, `reftable` for casing
  conflicts).
- Status checks use `git fetch --prune`, self-healing stale
  remote-tracking refs (fixes "some local refs could not be updated").
- Repo metadata (remote URL, branch, last commit) read via git2 instead of
  3 subprocesses per repo; `list_managed_repos_lite()` skips the `.git`
  size walk for commands that never display it.
- Audit phases 1-3 (secrets, git history, code patterns) run in parallel;
  the deps phase overlaps on the main thread.
- Config paths with a leading `~` expand on load (`scan_directories`,
  `repos_root`); partial config files merge with defaults
  (`#[serde(default)]`) instead of being discarded.
- `scanner/repo_manager.rs` split into a directory module
  (`mod`/`cache`/`meta`/`tests`) to stay under the 500-LOC convention.
- **`spark ingest` now delegates storage to TRS.** Digests live at
  `~/.trs/ingest/<owner>/<name>.md` (shared with `trs`), not
  `~/.config/spark/ingest/<host>/<owner>/<name>.md`. Run `trs ingest` or
  `spark ingest <name>` to regenerate after upgrading. See
  [`docs/dev/TRS_INTEGRATION.md`](docs/dev/TRS_INTEGRATION.md) for the split
  of responsibilities.
- `spark ingest --all` uses `trs --fresh` (git HEAD-based cache
  invalidation) instead of the prior 24-hour mtime heuristic.
- CI and the pre-push hook now run `cargo fmt --check` and
  `cargo clippy --all-targets -- -D warnings` — test code is linted too.
- Large source files split into directory modules (no public API changes).
  Every file is now under 500 LOC. Affected: `cli/audit`, `cli/repos`,
  `cli/ports`, `scanner/secret_scanner`, `scanner/code_patterns`,
  `scanner/dep_scanner`, `scanner/port_scanner`, `core/inventory`, `app`,
  `tui/model`, `tui/update`, `tui/widgets/scanner_view`,
  `tui/widgets/repo_manager_view`, `tui/widgets/audit_view`,
  `tui/widgets/system_view`.

### Removed
- Unused dependencies: `bytesize`; `chrono`'s `serde` feature;
  `x509-parser`'s `verify` feature. `cargo machete` now reports zero unused
  deps.
- Internal `has_ingest()` helper — unreachable after the TRS delegation
  refactor.

### Fixed
- Progress lines (`spark status`/`pull`/`report`/`ingest --all`) no longer
  leave leftovers or interleave: each update is a single write ending with
  erase-to-EOL, and progress is skipped entirely when stderr is not a
  terminal (clean piped logs).
- Status checks no longer report a stale "Up to date" when the fetch fails —
  a failed fetch with a clean 0/0 comparison now surfaces the error.
- `is_cache_valid` no longer underflows on clock skew (`saturating_sub`).
- `expand_tilde` handles a bare `~`; `config.example.toml`
  `max_scan_depth` now matches the real default (6).
- Docs pointed at `src/core/inventory.rs` (now a directory module) in
  CONTRIBUTING and ADDING_TOOLS; module paths and test counts refreshed.
- `docs/dev/INSTALLATION.md` no longer links to a nonexistent
  `config.example.toml`.
- `docs/dev/ARCHITECTURE.md` source tree matches the actual module layout
  after the splits.

## [0.5.1] — 2026-04-17

### Added
- UX improvements: audit folder picker, bulk system cleanup.
- `spark doctor` distinguishes auto-detected vs. configured repos root.

### Fixed
- `spark init` now creates `config.toml` with defaults when missing.
- Idempotent npm publish in release workflow (skips already-published
  versions instead of failing).
- `darwin-x64` cross-build uses `vendored-openssl` feature instead of
  relying on Rosetta Homebrew.
- Release workflow: sync version after artifact assembly so platform
  `package.json` isn't overwritten by the artifact copy.

## [0.5.0] — 2026-04-17

Initial public release.

### Added
- TUI with six tabs: Scanner, Repos, Ports, System, Audit, Updater.
- CLI commands: `spark clone / list / search / cd / rm / status / pull /
  tag / audit / ps / certs / ingest / root / config / doctor / init / agent /
  completions`.
- Scanner: git repo discovery, health scoring, artifact cleanup.
- Repo Manager: ghq-style clone/pull/status with 4-hour cache and tagging.
- Port Scanner: dev server discovery and kill (macOS + Linux).
- System Cleanup: Docker, dev caches, VMs, logs with safety guards.
- Security Audit: secrets, git history, OWASP Top 10 patterns, dependency
  vulnerabilities (OSV.dev + npm audit).
- Updater: manages updates for 55 developer tools.
- Multi-platform release pipeline: macOS arm64/x64, Linux x64; published to
  GitHub Releases and npm (`@dpeluche/spark`).

[Unreleased]: https://github.com/dPeluChe/spark/compare/v0.5.1...HEAD
[0.5.1]: https://github.com/dPeluChe/spark/compare/v0.5.0...v0.5.1
[0.5.0]: https://github.com/dPeluChe/spark/releases/tag/v0.5.0
