# SPARK - Installation Guide

## Quick Start

### Prerequisites
- **Rust toolchain** (rustc + cargo, install via [rustup](https://rustup.rs))
- **macOS or Linux** (macOS primary, Linux supported)
- Terminal with 256 color support

### Installation from Source

```bash
# 1. Clone the repository
cd /path/to/labs-spark

# 2. Build the release binary
cargo build --release

# 3. Install to local bin
mkdir -p ~/.local/bin
cp target/release/spark ~/.local/bin/spark

# 4. Verify installation
spark
```

### Shell Configuration

Add to your `~/.zshrc`:

```bash
alias spark='~/.local/bin/spark'
```

Then reload:

```bash
source ~/.zshrc
```

---

## Running SPARK

### Basic Usage

```bash
spark
```

This launches the interactive TUI dashboard in Updater mode.

### CLI Options

```bash
spark --scan-only    # Start directly in Scanner mode
spark --dry-run      # Preview updates without executing
```

---

## Configuration

SPARK reads from `~/.config/spark/config.toml`. See [config.example.toml](../config.example.toml) for all options.

---

## Updating SPARK

```bash
cd /path/to/labs-spark
cargo build --release
cp target/release/spark ~/.local/bin/spark
```

---

## Troubleshooting

### Issue: "command not found: spark"

Ensure `~/.local/bin` is in your PATH:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

### Issue: "could not open a new TTY"

SPARK requires an interactive terminal. Don't pipe or run in non-interactive contexts.

### Issue: Binary won't execute

```bash
chmod +x ~/.local/bin/spark
```

---

## Uninstallation

```bash
rm ~/.local/bin/spark
rm -rf ~/.config/spark/     # Remove config (optional)
```

---

## Build Dependencies

All managed by Cargo. Key crates:
- `ratatui` - TUI framework
- `tokio` - Async runtime
- `git2` - Git operations (requires libgit2/cmake)
- `crossterm` - Terminal manipulation

Run `cargo build` and Cargo handles everything.
