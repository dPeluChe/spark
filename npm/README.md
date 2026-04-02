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
- **Security Audit** — Secrets, OWASP patterns, dependency vulnerabilities
- **Certificate Scanner** — SSL/TLS certs, Keychain, loose keys in ~/
- **Updater** — Manage 44+ dev tools (brew, npm, macOS apps)

## CLI

```bash
spark                    # Open TUI
spark init               # Setup shell + completions
spark clone user/repo    # Clone to managed root
spark cd <name>          # Find repo path
spark search <query>     # Search repos (status, age, path)
spark list               # List repos (tree by host/owner)
spark status [query]     # Check which repos need pull
spark pull <query|all>   # Pull repos behind remote
spark audit [path]       # Security audit (4 phases)
spark audit --deps       # Dependency-only scan
spark certs              # SSL/TLS certificate health
spark root               # Show repos root
spark doctor             # Validate installation health
spark config             # Show/update config
spark agent              # AI agent integration tips
```

## Links

- [GitHub](https://github.com/dPeluChe/labs-spark)
- [Documentation](https://github.com/dPeluChe/labs-spark/blob/main/CLAUDE.md)
