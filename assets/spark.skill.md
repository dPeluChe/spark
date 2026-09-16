---
name: spark
description: >
  DevOps operations via the SPARK CLI. Use when the user asks about repo status,
  security auditing, system cleanup, port conflicts, SSL certificates, or managing
  dev tool updates. Trigger on: "are my repos up to date", "check for secrets",
  "audit the code", "clean up disk", "what port is running on", "check certs",
  "update my tools", "which repos need a pull", "find repo", "tag repos",
  "what needs attention", "fleet status", "triage".
  Also trigger before commits (security check) and before deploys (status check).
version: 1.1.0
---

# spark — DevOps CLI

SPARK is the local DevOps platform. All commands run on the user's machine. No auth needed except `spark status`/`pull` (network) and `spark audit` dep scan (OSV.dev).

## TRIGGER when

- User asks about repo sync, status, or needing to pull
- User asks "what needs attention" / wants a fleet overview (disk, ports, tools, security)
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

### Start here: fleet triage
```bash
spark report --json      # ONE call: repos, disk, ports, tools, security
```
Parse the JSON and act on what it reports:
- `repos.behind > 0` → offer `spark pull all`
- `disk.artifacts_bytes`/`disk.system_bytes` large → point at `spark system` (TUI cleanup)
- `ports` non-empty → `spark ps` for detail, `spark ps <name> --kill` to stop
- `tools.outdated > 0` → TUI Updater tab
- `security.total > 0` or stale → `spark audit`

`spark report` (human output) is the same data for showing the user directly.
Use `--fresh` only when the user wants live numbers (network + ~30s).

### Before a commit — security check
```bash
spark audit              # full: secrets + git history + OWASP + deps (OSV.dev)
spark audit --offline    # faster, no network, skips dep scan
spark audit --json       # machine-readable findings (json_version: 1)
```
Exit code is 1 when findings exist (0 clean) — use it as a gate in scripts/CI.
Look for CRITICAL findings in the output. If found, report them and suggest fixes.

### Are repos up to date?
```bash
spark status             # shows which repos are Behind / Diverged / Dirty / Up-to-date
spark status --json      # per-repo kind/ahead/behind/dirty + summary counters
spark status --exit-code # exit 1 when any repo needs attention (scripts/CI)
spark status --tag work  # filter by tag
spark pull all           # pull all repos behind remote (ff-only, safe)
spark pull all --tag work
```
`spark pull all` buckets its output: pulled / up to date / skipped (dirty, ahead,
diverged) / errors with one-line remediation hints. Dirty and diverged repos are
never force-touched.

### Find / navigate a repo
```bash
spark search <query>     # shows status, commit age, branch, path
spark list               # tree view by host/owner
spark list --json        # full inventory (json_version: 1)
spark cd <name>          # prints path (use with spark-cd shell function)
spark-cd <name>          # navigates shell to the repo (requires spark init)
```

### Free up disk space
```bash
spark report             # shows total recoverable (artifacts + system)
spark                    # TUI → System tab → press ENTER on scan → clean caches
```
Or suggest the user open the TUI and navigate to System tab.

### Port conflict / what's running
```bash
spark ps                    # dev server ports (pid, process, runtime, project)
spark ps --json             # machine-readable (kind: dev/system/service/app)
spark ps --all              # all ports: SYSTEM macOS / SERVICES / APPS sections
spark ps postino            # search processes by name, shows their ports
spark ps postino --json     # processes with their ports as JSON
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
spark report --fresh     # counts outdated tools (network)
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
**TRS owns digest generation and storage** (`~/.trs/ingest/`). SPARK adds a fleet-level
wrapper: resolves repo by name and runs trs inside the repo path. Shared storage — both
tools see the same digests.

**Fleet operations (SPARK):**
```bash
spark ingest --all                 # batch all managed repos (trs --fresh skips unchanged)
spark ingest                       # list with fleet awareness (managed vs external)
spark ingest <repo> --read         # print a managed repo's digest to stdout
spark ingest <repo>                # generate digest for a managed repo by name (no cd)
```

**Single-repo operations (TRS directly, preferred when inside a repo):**
```bash
trs ingest                         # digest current directory
trs ingest --list                  # full TRS catalog (includes external repos)
trs ingest --read <name>           # read back a stored digest
trs ingest --budget 32k            # fit to context window
trs ingest --changed               # only uncommitted files — fast mid-session
trs ingest --since HEAD~5          # only last 5 commits
trs ingest --deps                  # dependency graph only — no file content
trs ingest -l aggressive           # aggressive compression (~93% reduction)
trs ingest --fresh                 # skip regen if HEAD unchanged
```

**Rule of thumb:** many repos → `spark ingest --all`. One repo → `trs ingest` from inside.

Check trs is installed: `which trs` — if missing: `npm install -g @dpeluche/trs`

---

## Machine-readable outputs (prefer these over parsing prose)

All carry `json_version: 1` (stable contract, see docs/dev/ROADMAP.md in the repo):

| Command | Returns | Exit code |
|---------|---------|-----------|
| `spark report --json` | repos summary, disk, ports, tools, security | 0 |
| `spark status --json` | per-repo `{status, ahead, behind, dirty, path, tags, ...}` + summary | 0 (1 with `--exit-code` when any repo needs attention) |
| `spark list --json` | repo inventory (host/owner/name/path/branch/last_commit/tags) | 0 |
| `spark ps --json` | ports with `kind` (dev/system/service/app); processes in query mode | 0 |
| `spark audit --json` | redacted findings (secrets/history/patterns/deps) + summary | 1 when findings, 0 clean |

Progress/spinner text goes to stderr — stdout is pure JSON, safe to pipe.

---

## Key facts for the agent

- **Repos root**: auto-detected from ghq root or `~/ghq`. Check with `spark config`.
- **Audit ignores**: `.sparkauditignore` suppresses reviewed findings (`spark audit --init` creates one).
- **Status cache**: 4h TTL. `spark report`/`spark status` use it by default; `--fresh` forces network checks.
- **Last audit**: `spark audit` writes `last_audit.json` (timestamp, path, findings count) — `spark report` shows it.
- **spark-cd**: shell function installed by `spark init` — needed for `spark-cd <name>` navigation.
- **TUI tabs**: Scanner → Repos → Ports → System → Audit → Updater (TAB cycles, q goes back).
- **Ingest output**: `~/.trs/ingest/<owner>/<repo>.md` — shared storage with TRS, single source of truth.
- **Ingest backend**: TRS is the sole backend (owns generation + storage). SPARK wraps it for fleet operations. See [docs/dev/TRS_INTEGRATION.md](https://github.com/dPeluChe/spark/blob/main/docs/dev/TRS_INTEGRATION.md).
- **Fleet vs instance**: `spark ingest --all` for batch, `trs ingest` for single-repo/current-dir work.

## Installation check
```bash
which spark && spark --version   # verify installed
spark doctor                     # full health check
spark init                       # setup: shell function, completions, AI skills
```
