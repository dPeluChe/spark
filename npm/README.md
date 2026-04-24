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
- **Repo Manager** — ghq-compatible clone/pull/status + tagging
- **Port Scanner** — Find & kill forgotten dev servers (`spark ps`)
- **System Cleanup** — Docker, caches (brew/npm/pip/cargo), VMs, logs
- **Security Audit** — Secrets, OWASP patterns, dependency vulnerabilities
- **Certificate Scanner** — SSL/TLS certs, Keychain, loose keys in ~/
- **Updater** — Manage 55 dev tools (brew, npm, macOS apps)

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
spark tag add <r> <tag>  # Tag repos for group management
spark pull all --tag <t> # Pull repos by tag
spark ps                 # Dev servers (pid, runtime, project)
spark ps --kill <target> # Kill by port, PID, or name
spark audit [path]       # Security audit (4 phases)
spark audit --deps       # Dependency-only scan
spark certs              # SSL/TLS certificate health
spark root               # Show repos root
spark doctor             # Validate installation health
spark config             # Show/update config
spark agent              # AI agent integration tips
spark ingest --all       # LLM digests for all managed repos (wraps trs)
spark ingest             # List digests with fleet awareness
```

## Links

- [GitHub](https://github.com/dPeluChe/spark)
- [Landing page](https://dpeluche.github.io/spark/)
- [Contributing](https://github.com/dPeluChe/spark/blob/main/CONTRIBUTING.md)

---

A product by [Iteris](https://iteris.tech) · Published and maintained by [@dPeluChe](https://github.com/dPeluChe)
