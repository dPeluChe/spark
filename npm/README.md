# SPARK — Developer Operations Platform

A high-performance TUI for managing your entire dev environment.

```
   _____ ____  ___  ____  __ __
  / ___// __ \/   |/ __ \/ //_/
  \__ \/ /_/ / /| / /_/ / ,<
 ___/ / ____/ ___ / _, _/ /| |
/____/_/   /_/  |/_/ |_/_/ |_|
```

## Install

```bash
# npm
npm install -g @dpeluche/spark

# npx (run without installing)
npx @dpeluche/spark

# Setup shell integration
spark init
```

## Features

- **Scanner** — Discover git repos, health scoring, artifact cleanup
- **Repo Manager** — ghq-compatible clone/pull/status (42+ repos)
- **Port Scanner** — Find & kill forgotten dev servers
- **System Cleanup** — Docker, caches (brew/npm/pip/cargo), VMs, logs
- **Updater** — Manage 44+ dev tools (brew, npm, macOS apps)

## CLI

```bash
spark                    # Open TUI
spark init               # Setup shell + completions
spark clone user/repo    # Clone to managed root
spark cd <name>          # Find repo path
spark search <query>     # Search repos
spark list               # List managed repos
spark root               # Show repos root
spark config             # Show/update config
spark agent              # AI agent integration tips
```

## Links

- [GitHub](https://github.com/dPeluChe/labs-spark)
- [Documentation](https://github.com/dPeluChe/labs-spark/blob/main/CLAUDE.md)
