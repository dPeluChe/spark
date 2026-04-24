# Contributing to SPARK

Thanks for your interest in contributing. SPARK is a Rust TUI built for developer operations — the bar for code quality is high but the onboarding is straightforward.

## Quick start

```bash
git clone https://github.com/dPeluChe/spark.git
cd spark
cargo build          # dev build
cargo run            # run the TUI
cargo test           # 127 tests
```

All three must pass before opening a PR.

## Before you start

- Check [docs/dev/TASK_TODO.md](docs/dev/TASK_TODO.md) for planned work
- Open an issue before starting large changes — alignment saves time
- For bugs, include `spark doctor` output and the steps to reproduce

## Code standards and architecture

See [docs/dev/DEV_GUIDELINES.md](docs/dev/DEV_GUIDELINES.md) for code standards, patterns, and conventions.

See [docs/dev/ARCHITECTURE.md](docs/dev/ARCHITECTURE.md) for the full codebase map.

## Adding a new tool to the Updater

See [docs/dev/ADDING_TOOLS.md](docs/dev/ADDING_TOOLS.md). The short version: add an entry to `src/core/inventory.rs` with the tool name, category, detection command, and update method. No other files need changing for most tools.

## Releasing a new version (maintainers)

1. Move entries under `[Unreleased]` in [CHANGELOG.md](CHANGELOG.md) to a new
   dated section `[X.Y.Z] — YYYY-MM-DD`, and update the compare links at the
   bottom of the file.
2. Bump all 5 manifests atomically:
   ```bash
   scripts/version.sh bump X.Y.Z
   ```
   This updates `Cargo.toml`, `npm/package.json` (version +
   `optionalDependencies` pin), the 3 `npm/platforms/*/package.json`, and
   regenerates `Cargo.lock`.
3. Commit and tag:
   ```bash
   git add Cargo.toml Cargo.lock npm/ CHANGELOG.md
   git commit -m "Release: vX.Y.Z"
   git tag vX.Y.Z
   git push origin main --follow-tags
   ```
4. The `Release` workflow runs on the tag push. It first verifies the tag
   and manifests agree (`scripts/version.sh check` + tag comparison), then
   builds the 3 platform binaries, creates the GitHub Release, and publishes
   to npm.

Run `scripts/version.sh check` anytime to confirm all manifests are in sync.

## Pull request checklist

- [ ] `cargo test` passes
- [ ] `cargo clippy --all-targets -- -D warnings` clean
- [ ] `cargo fmt --check` passes
- [ ] New behavior has at least one test
- [ ] PR description explains *why*, not just *what*
- [ ] Screenshots/output for TUI changes

## Commit style

```
Type: short description (imperative, under 72 chars)

Optional body explaining why, not what.
```

Types: `Feat`, `Fix`, `Chore`, `Docs`, `Refactor`, `Perf`, `Test`

## Reporting issues

- **Bug**: include `spark --version`, `spark doctor`, and steps to reproduce
- **Feature request**: describe the use case, not just the solution
- **Security**: email directly rather than opening a public issue

## License

By contributing, you agree your code will be licensed under the project's [MIT License](LICENSE).
