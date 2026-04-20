# SPARK — Installation Guide

## Install

| Method | Command |
|--------|---------|
| **npm** (recommended) | `npm install -g @dpeluche/spark` |
| **npx** (no install) | `npx @dpeluche/spark` |
| **curl** | `curl -fsSL https://raw.githubusercontent.com/dPeluChe/spark/main/scripts/install.sh \| sh` |
| **cargo** | `cargo install --git https://github.com/dPeluChe/spark` |
| **Binary** | [GitHub Releases](https://github.com/dPeluChe/spark/releases) — macOS arm64/x64, Linux x64 |

After installing, run setup:

```bash
spark init    # shell integration (spark-cd), completions, config
spark doctor  # validate everything is working
spark         # open the TUI
```

---

## Build from source

For contributors or if you want the latest unreleased code:

```bash
git clone https://github.com/dPeluChe/spark.git
cd spark
cargo build --release
cargo install --path .    # installs to ~/.cargo/bin/spark (already in PATH via rustup)
```

Verify:

```bash
spark --version
spark doctor
```

---

## Shell setup

`spark init` handles everything:
- Adds `spark-cd` function to your shell rc (zsh/bash/fish)
- Generates completions
- Creates `~/.config/spark/config.toml` with defaults

`spark-cd` lets you navigate to managed repos:

```bash
spark-cd zed        # cd to the zed repo
spark-cd api        # cd to any repo matching 'api'
```

---

## Configuration

Config file: `~/.config/spark/config.toml`  
macOS alternate: `~/Library/Application Support/spark/config.toml`

See [config.example.toml](../../config.example.toml) for all options. Key fields:

```toml
repos_root = "~/repos"          # root for managed repos (ghq-compatible)
stale_threshold_days = 90       # days before a repo is considered stale
use_trash = true                # use OS trash instead of permanent delete
max_scan_depth = 6              # recursion depth for repo scanner
```

---

## Troubleshooting

**`spark: command not found`**  
The binary isn't in PATH. Prefer reinstalling via npm (`npm install -g @dpeluche/spark`) which places the binary in npm's global bin (already in PATH). If building from source, use `cargo install --path .` — rustup adds `~/.cargo/bin` to PATH automatically.

**`spark: platform binary not found`**  
npm wrapper couldn't find the platform binary. Run `npm install -g @dpeluche/spark` again to re-download. Check `spark doctor` for details.

**`spark doctor` fails a check**  
`spark doctor` reports what's missing with suggestions. Most issues are fixed by running `spark init` again.

**TUI won't open / display is garbled**  
SPARK requires a real interactive terminal with 256-color support. Don't pipe or run in non-interactive contexts (CI, `ssh -t` required for remote sessions).

---

## Uninstall

```bash
npm uninstall -g @dpeluche/spark   # if installed via npm
# or
cargo uninstall spark              # if installed via cargo

rm -rf ~/.config/spark/            # remove config and cache (optional)
```
