---
name: spark
description: >
  DevOps operations via the SPARK CLI. Use when the user asks about repo status,
  security auditing, system cleanup, port conflicts, SSL certificates, or managing
  dev tool updates. Trigger on: "are my repos up to date", "check for secrets",
  "audit the code", "clean up disk", "what port is running on", "check certs",
  "update my tools", "which repos need a pull", "find repo", "tag repos".
  Also trigger before commits (security check) and before deploys (status check).
version: 1.0.0
---

# spark — DevOps CLI

SPARK is the local DevOps platform. All commands run on the user's machine. No auth needed except `spark status`/`pull` (network) and `spark audit` dep scan (OSV.dev).

## TRIGGER when

- User asks about repo sync, status, or needing to pull
- User wants a security/secrets audit before committing or reviewing code
- User mentions disk space, caches, Docker cleanup
- User has a port conflict or wants to see what's running
- User asks about SSL/TLS certificates
- User wants to update dev tools (brew, npm globals, IDEs, runtimes)
- User asks to find, navigate to, or group repos by tag

## SKIP when

- User is asking about a specific app's business logic (not DevOps)
- User is in a non-dev context (writing, design)
- spark is not installed (`which spark` fails)

---

## Workflows

### Before a commit — security check
```bash
spark audit              # full: secrets + git history + OWASP + deps (OSV.dev)
spark audit --offline    # faster, no network, skips dep scan
```
Look for CRITICAL findings in the output. If found, report them and suggest fixes.

### Are repos up to date?
```bash
spark status             # shows which repos are Behind / Diverged / Up-to-date
spark status --tag work  # filter by tag
spark pull all           # pull all repos behind remote (ff-only, safe)
spark pull all --tag work
```

### Find / navigate a repo
```bash
spark search <query>     # shows status, commit age, branch, path
spark list               # tree view by host/owner
spark cd <name>          # prints path (use with spark-cd shell function)
spark-cd <name>          # navigates shell to the repo (requires spark init)
```

### Free up disk space
```bash
spark                    # TUI → System tab → press ENTER on scan → clean caches
```
Or suggest the user open the TUI and navigate to System tab.

### Port conflict / what's running
```bash
spark ps                    # dev server ports (pid, process, runtime, project)
spark ps --all              # all ports: SYSTEM macOS / SERVICES / APPS sections
spark ps postino            # search processes by name, shows their ports
spark ps --kill 3000        # kill process on port 3000 (interactive confirm)
spark ps --kill postino     # kill by name (interactive confirm)
spark ps postino --kill     # kill by name non-interactive (exit 0=killed, 1=not found)
spark ps 3000 --kill        # kill port 3000 non-interactive (for scripts and agents)
```

### SSL certificates
```bash
spark certs              # all certs: Keychain + files + ~/home
spark certs --expired    # show only expired
spark certs --summary    # counts only, no detail
```

### Update dev tools
```bash
spark                    # TUI → Updater tab — shows outdated tools, SPACE to select, u to update
```

### Group repos by project/client
```bash
spark tag add <repo> <tag>    # e.g. spark tag add labs-spark work
spark tag list                # all tags
spark tag list work           # repos in 'work' tag
spark status --tag work       # status for tagged group
spark pull all --tag work     # pull tagged group
```

### LLM context for a repo
```bash
spark ingest <repo>           # generates repomix markdown at ~/.config/spark/ingest/
spark ingest <repo> --compress  # ~70% token reduction via tree-sitter
spark ingest --all            # batch all managed repos
```

---

## Key facts for the agent

- **Repos root**: auto-detected from ghq root or `~/ghq`. Check with `spark config`.
- **Audit ignores**: `.sparkauditignore` suppresses reviewed findings (`spark audit --init` creates one).
- **Status cache**: 4h cache for `spark status`. Press `r` in TUI or run `spark status` fresh.
- **spark-cd**: shell function installed by `spark init` — needed for `spark-cd <name>` navigation.
- **TUI tabs**: Scanner → Repos → Ports → System → Audit → Updater (TAB cycles, q goes back).
- **Ingest output**: `~/.config/spark/ingest/<host>/<owner>/<repo>.md` — markdown for LLM context.

## Installation check
```bash
which spark && spark --version   # verify installed
spark doctor                     # full health check
spark init                       # setup: shell function, completions, AI skills
```
