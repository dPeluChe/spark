# SPARK — Dev Guidelines

Code standards for contributing to SPARK. The pre-push hook enforces the mechanical checks automatically; this document covers the reasoning and conventions that tools can't catch.

## Automated checks (pre-push hook)

```bash
cargo test                  # all tests must pass
cargo clippy -- -D warnings # no warnings allowed
cargo fmt -- --check        # formatting must match rustfmt defaults
```

Run all three before opening a PR.

---

## Error handling

- **No `unwrap()` on user-facing paths** — use `?` or explicit error handling. Panics crash the TUI with no useful message.
- **No `expect()` without justification** — if you know something can't fail, say why in a comment.
- Prefer `?` propagation in CLI commands and scanner functions where the caller can surface the error.
- Use `color-eyre` for top-level error display in CLI paths.

## Paths and directories

- **No hardcoded paths** — use the `dirs` crate for home (`dirs::home_dir()`), config (`dirs::config_dir()`), data dirs.
- **No string concatenation for paths** — use `PathBuf` and `.join()`.
- Use `utils::fs::expand_tilde()` when accepting user-supplied paths (handles `~/` prefix).
- Use `scanner::common::shorten_path()` for display (replaces home with `~`).

## Where new code goes

| What you're adding | Where it goes |
|--------------------|---------------|
| New data scanner / analysis | `src/scanner/` |
| New render function / widget | `src/tui/widgets/` |
| New `spark <subcommand>` | `src/cli/` |
| New async utility | `src/utils/` |

## Adding a new TUI tab or state

New TUI states require entries in all four places:

1. `ScannerState` enum in `src/tui/model.rs`
2. Match arm in `src/tui/scanner_keys/mod.rs` (route to correct handler file)
3. New handler file or arm in `src/tui/scanner_keys/<tab>.rs`
4. Render arm in `src/tui/view.rs` (tab bar + render dispatcher)

Missing any one of these produces a compile error (`non-exhaustive pattern`). That's intentional.

## TUI rendering rules

- **Render functions must be pure** — no state mutation inside widget code. Read from model, write to `Frame`. Side effects belong in `app.rs`.
- Each widget manages its own layout (`Layout::default().split()`). Don't pass pre-computed layout from the caller.
- Use `styles.rs` color constants — don't hardcode color values in widget files.

## Tests

- Tests live next to the code they test: `#[cfg(test)]` module at the bottom of the same file.
- New behavior needs at least one test. Untestable TUI rendering is fine to skip; data logic is not.
- Don't mock the filesystem or git — use real temp directories (`tempfile` crate) for integration-level tests.

## Concurrency

- Never block the tokio runtime with synchronous I/O. Use `spawn_blocking` for any blocking call.
- State is only modified in the event loop via `AppMessage`. Don't mutate `App` from inside spawned tasks.
- Use `mpsc::unbounded_channel` — already established; don't introduce new channel types without discussion.

## Logging / debugging

Use the debug log utility in `utils/shell.rs` for diagnostic output during development. The log goes to `spark_debug.log` and is `.gitignore`d. Never use `println!` in production paths — it corrupts the TUI output.

## Commit style

```
Type: short description (imperative, under 72 chars)

Optional body explaining why, not what.
```

Types: `Feat`, `Fix`, `Chore`, `Docs`, `Refactor`, `Perf`, `Test`
