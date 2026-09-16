# SPARK — Roadmap: Agent-First Local DevOps

## Focus

SPARK is the **local DevOps toolkit for developers and their coding agents**. One CLI
that answers "what needs attention on this machine right now?" and can be driven by
both humans (TUI, formatted output) and agents (JSON, exit codes, skills).

Not competing with single-repo TUIs (lazygit, gitui) or single-purpose cleaners
(mole). SPARK's identity is the **fleet view**: many repos, disk, ports, tools,
security — aggregated in one place.

## Why this focus (evidence, 2026-09)

- External adoption of the human-first TUI is ~zero (0 stars, 14 npm downloads/month)
  while `trs` (machine-readable digests for agents, same author) has 840 downloads/month.
  The audience that consumes these tools is agents; the format that travels is JSON.
- The user's own agents already operate SPARK daily through the CLI + skill — that is
  the living use case to optimize for.
- Market check: MCP/DevOps tooling is crowded for cloud (GitHub, k8s, CI) but
  **local-machine devops for agents is nearly empty**; existing macOS MCP servers are
  generic OS automation, not developer workflows.

## Principles

1. **CLI-first.** Everything the agent needs is a command with a stable JSON contract.
   No daemon, no background service required.
2. **JSON is API.** Every `--json` output is a versioned contract (see schemas below).
   Breaking changes require a `json_version` bump.
3. **Cache-friendly.** Interactive speed comes from the 4h status cache and parallel
   pools (see `STATUS_CONCURRENCY`); network refresh is always opt-in (`--fresh`).
4. **Human and agent share one code path.** The JSON is a rendering of the same data
   the TUI shows; no parallel implementations.

---

## Phase 0 — Machine-readable outputs

**Status**: shipped 2026-09-16.
**Goal**: agents and CI can consume SPARK without parsing prose.

### Deliverables

1. `--json` flag for `status`, `list`, `ps`, `audit`.
2. Exit codes:
   - `audit`: exit 1 when findings exist (CI gate), 0 clean.
   - `status --exit-code`: exit 1 when any repo needs attention (opt-in, keeps
     interactive default of 0).
3. `RepoStatus::Dirty` carries `ahead`/`behind` counts so agents can distinguish
   "dirty but current" from "dirty and 3 behind".

### JSON schemas (v1)

`spark status --json`:

```json
{
  "json_version": 1,
  "generated_at": "2026-09-16T18:00:00Z",
  "summary": {
    "total": 145, "up_to_date": 127, "behind": 12,
    "ahead": 0, "dirty": 4, "diverged": 2, "error": 0
  },
  "repos": [
    {
      "host": "github.com", "owner": "dPeluChe", "name": "spark",
      "path": "/Users/x/ghq/github.com/dPeluChe/spark",
      "branch": "main", "status": "behind",
      "ahead": 0, "behind": 3, "dirty": false,
      "last_commit": "2d ago", "tags": ["work"]
    }
  ]
}
```

`status` string enum: `up_to_date | behind | ahead | diverged | dirty | error`.
`ahead`/`behind` are always numbers; `dirty` always a bool, independent of `status`.

`spark list --json`: `{json_version, repos: [{host, owner, name, path, branch, last_commit, tags}]}`.

`spark ps --json`: `{json_version, ports: [{port, pid, process, runtime, project, kind}]}`
where `kind` is `dev | system | service | app` (mirrors the TUI grouping).

`spark audit --json`: `{json_version, path, generated_at, summary: {total, secrets, history, patterns, deps}, secrets: [...], history: [...], patterns: [...], deps: {...}}` —
reuses the scanner finding structs (add `Serialize` derives).

### Acceptance criteria

- `--json` output validates as JSON, contains `json_version: 1`, and is stable
  across runs for unchanged state.
- `spark audit --json | jq .summary.total` works; `spark audit; echo $?` returns 1
  on findings, 0 clean.
- Dirty repos show real ahead/behind counts in `status` and pull classification.
- Tests: serde round-trip per schema + exit-code integration tests.

---

## Phase 1 — `spark report` (the fleet view)

**Status**: shipped 2026-09-16.
**Goal**: one command that answers "what needs attention?" for humans and agents.

```
$ spark report
  SPARK Fleet Report · 145 repos · status cached 2h ago

  Repos       127 up to date · 12 behind · 4 dirty · 2 diverged
              → spark pull all
  Disk        18.2 GB recoverable
              → 14.1 GB artifacts in 23 repos · 2.8 GB docker · 1.3 GB caches
              → spark system
  Ports       3 dev servers (3000, 5432, 8080)  → spark ps
  Tools       5 outdated  → TUI Updater
  Security    last audit 2d ago · 3 findings  → spark audit
```

### Flags

- default: fast (status cache, local scans, cached tool state)
- `--fresh`: re-fetch repo statuses and tool versions
- `--json`: same schema idea as `{json_version, repos, disk, ports, tools, security}`

### Data sources (all existing)

| Section | Source |
|---------|--------|
| Repos | `repo_manager` status cache + parallel pool |
| Disk artifacts | `space_analyzer::find_artifacts` per repo, parallelized with the 8-worker pool |
| Disk system | `system_cleaner::scan_system()` (docker/caches/vms/logs) |
| Ports | `port_scanner::scan_ports()` |
| Tools | updater detector (cached; network only with `--fresh`) |
| Security | `last_audit.json` summary persisted by `spark audit` |

### Acceptance criteria

- Cold `spark report` on the 145-repo fleet completes in < 60s; warm (cached) < 5s.
- Every section degrades gracefully (missing docker, no audit yet, etc.).
- `--json` mirrors the human sections 1:1.

---

## Phase 2 — Agent loop closure

**Status**: shipped 2026-09-16.

- `spark.skill.md` (assets/, v1.1.0) teaches agents: start with
  `spark report --json` for triage, prefer `--json` over parsing prose, use
  `audit` exit codes as gates.
- `spark audit` persists `last_audit.json` (summary only, no findings bodies) —
  shipped with Phase 1.
- README / README.es document the agent workflow; ARCHITECTURE and CLAUDE.md
  cover `report.rs` and the `json.rs` contracts.

---

## Parked (with revisit criteria)

| Item | Revisit when |
|------|--------------|
| MCP server | Distribution to third-party agents without the skill installed, or clear external adoption signal. For in-house agents the CLI + skill covers it with fewer moving parts. |
| Cross-platform (linux-arm64, Windows port/cert scanner) | External adoption signal; today the fleet is macOS. |
| Homebrew tap, marketing push | After the agent-first identity ships (Phases 0-2). |

## Non-goals

- Interactive git operations (staging, rebase) — lazygit's territory.
- Background daemon / always-on service — cache + explicit `--fresh` instead.
- Cloud/remote infra management — local machine only.
