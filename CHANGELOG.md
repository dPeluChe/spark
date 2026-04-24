# Changelog

All notable changes to SPARK are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
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

### Added
- [`docs/dev/TRS_INTEGRATION.md`](docs/dev/TRS_INTEGRATION.md) — division of
  responsibilities between SPARK (fleet layer) and TRS (digest generator + storage)
  with lessons learned from the overlap.
- `scripts/version.sh` — atomic bump + consistency check across the 5
  version fields (Cargo.toml + 4 npm manifests). Invoked by release CI.
- CI gate (`verify-versions` job) that blocks the release if the git tag
  disagrees with Cargo.toml or if any manifest drifts.
- `CHANGELOG.md` (this file).

### Removed
- Unused dependencies: `bytesize`; `chrono`'s `serde` feature;
  `x509-parser`'s `verify` feature. `cargo machete` now reports zero unused
  deps.
- Internal `has_ingest()` helper — unreachable after the TRS delegation
  refactor.

### Fixed
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
