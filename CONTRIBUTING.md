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
