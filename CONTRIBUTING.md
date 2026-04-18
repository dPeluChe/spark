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

- Check [docs/TASK_TODO.md](docs/TASK_TODO.md) for planned work
- Open an issue before starting large changes — alignment saves time
- For bugs, include `spark doctor` output and the steps to reproduce

## Code standards

**The pre-push hook enforces these automatically:**

```bash
cargo test                  # all tests must pass
cargo clippy -- -D warnings # no warnings allowed
cargo fmt -- --check        # formatting must match rustfmt defaults
```

**What we care about:**

- No `unwrap()` on user-facing paths — use `?` or explicit error handling
- No hardcoded paths — use `dirs::` crate for home, config, data dirs
- New modules go in `src/scanner/` (data) or `src/tui/widgets/` (render) or `src/cli/` (commands)
- New TUI states need entries in: `ScannerState` enum, `scanner_keys/mod.rs`, `scanner_view.rs`, `view.rs` tab bar
- Keep render functions pure — no state mutation in widget code
- Tests live next to the code they test (`#[cfg(test)]` at bottom of the file)

## Architecture overview

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full map. The short version:

```
src/
├── app.rs          — event loop, Action dispatch via mpsc channels
├── config.rs       — SparkConfig, auto-detection of repos root
├── cli/            — all spark <subcommand> implementations
├── scanner/        — data layer: repos, ports, system, audit, certs
├── tui/
│   ├── model.rs    — all state (App, ScannerModel, AuditModel, ...)
│   ├── update.rs   — Action enum + message handling
│   ├── view.rs     — tab bar + render dispatcher
│   ├── scanner_keys/ — key handlers split by tab
│   └── widgets/    — all render functions
└── utils/          — fs helpers (format_size, expand_tilde, dir_size)
```

The TUI follows a strict MVU pattern:
1. Key event → `scanner_keys/` handler returns `Option<Action>`
2. Action dispatched via `mpsc` channel to `app.rs`
3. `app.rs` spawns async tasks, sends `AppMessage` back
4. Model updates, next frame renders

## Adding a new tool to the Updater

See [docs/ADDING_TOOLS.md](docs/ADDING_TOOLS.md). The short version: add an entry to `src/core/inventory.rs` with the tool name, category, detection command, and update method. No other files need changing for most tools.

## Pull request checklist

- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt -- --check` passes
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
