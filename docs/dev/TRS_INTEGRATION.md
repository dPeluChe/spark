# SPARK ↔ TRS — Integration & Lessons Learned

How `spark ingest` relates to [TRS](https://github.com/dPeluChe/trs), what each tool owns, and the refactor direction derived from analyzing the overlap.

---

## TL;DR

**TRS owns digest generation and storage. SPARK owns the fleet layer on top.**

- Generating, compressing, caching, and listing digests → TRS.
- Iterating over managed repos (`--all`) and cross-referencing with repo status → SPARK.
- Both tools are aware of each other: `trs ingest owner/repo` uses spark's repo dir when spark is installed; `spark ingest` delegates all generation to trs.

---

## Who does what

| Responsibility | Owner | Where |
|---------------|-------|-------|
| Walk the repo, parse files, apply compression | **TRS** | `trs ingest` |
| Token budget, `--changed`, `--since`, `--deps`, `--fresh` | **TRS** | `trs ingest --budget/--changed/--since/--deps/--fresh` |
| Persistent digest storage | **TRS** | `~/.trs/ingest/<owner>/<name>.md` |
| Catalog listing with age, size, tokens, commits-behind | **TRS** | `trs ingest --list` |
| Read back a stored digest | **TRS** | `trs ingest --read <name>` |
| Cache invalidation on HEAD change | **TRS** | `--fresh` flag, stored HEAD |
| Name → path resolution across managed repos | **SPARK** | `repo_manager::list_managed_repos` |
| Fleet-level batch ingest across N repos | **SPARK** | `spark ingest --all` |
| Cross-referencing digest freshness with `spark status` cache | **SPARK** | `repo_status_cache` + digest mtime |
| `pull + ingest` combos (future `spark sync`) | **SPARK** | not yet implemented |

---

## When to use what

| Situation | Use |
|-----------|-----|
| Digest of the current directory | `trs ingest` |
| Digest of an external repo (clone-and-discard) | `trs ingest owner/repo --tmp` |
| Digest of a repo you tracked with spark | `trs ingest` (from inside) — or `spark ingest foo` once delegation lands |
| See all digests across everything you have | `trs ingest --list` |
| Regenerate digests for **your whole fleet** | `spark ingest --all` |
| Quick one-off with `--budget`, `--changed`, etc. | `trs ingest` directly |

Principle: **if you care about one repo → use TRS. If you care about many repos → use SPARK.**

---

## The overlap we reimplemented (and shouldn't have)

Today `spark ingest` (see `src/scanner/repo_ingest.rs` and `src/cli/ingest.rs`) duplicates several capabilities TRS already provides natively. The current implementation:

1. **Forces `trs` to write into spark's own directory** via `-o ~/.config/spark/ingest/<host>/<owner>/<name>.md` — explicitly bypassing `~/.trs/ingest/` (see the comment `// Write directly to spark's path — no shadow save in ~/.trs/ingest/` in `repo_ingest.rs`).
2. **Maintains its own catalog** in `cmd_ingest_list` instead of calling `trs ingest --list`.
3. **Implements its own `--read`** in `cmd_ingest_read` instead of calling `trs ingest --read <name>`.
4. **Cross-references digest age with `repo_status_cache`** for stale detection — TRS already reports "N commits behind" natively in its listing.
5. **Forwards `--budget/--changed/--since/--deps/--compress/--fresh` flags** as pure passthrough, adding no value beyond the passthrough itself.

The only genuine SPARK-level feature is `spark ingest --all` (batch loop over managed repos with skip-if-recent). That's what justifies the existence of `spark ingest` at all.

---

## Architectural principle

> **TRS is the source of truth for digests. SPARK adds fleet-level orchestration on top.**

Analogous to `ghq` vs `git clone`: git owns the repo state, ghq orchestrates a fleet of them. Same split here — trs owns the digest, spark orchestrates a fleet of them.

Corollaries:

- **One storage location.** Digests live in `~/.trs/ingest/`. SPARK does not have its own directory.
- **One catalog.** `trs ingest --list` is the canonical view. `spark ingest` (no args) should delegate or enrich.
- **Passthrough flags deprecate in spark.** If the user needs `--budget`, they can run `trs ingest` directly — no value in going through spark.
- **Only fleet operations stay in spark.** `spark ingest --all` is the one primitive worth keeping.

---

## Refactor direction

Tracked in [TASK_TODO.md](TASK_TODO.md) as "Refactor `spark ingest` to delegate to TRS". Summary:

1. **Remove `-o` override** in `repo_ingest::generate_ingest` → let TRS save to its own location.
2. **Delegate `cmd_ingest_list`** → shell out to `trs ingest --list` (optionally parse + enrich with spark-specific info).
3. **Delegate `cmd_ingest_read`** → shell out to `trs ingest --read <name>`.
4. **Deprecate passthrough flags** on `spark ingest <repo>` with deprecation warnings pointing users to `trs ingest` directly. Keep `--all` as the sole spark-level flag.
5. **Update `README.md`, `npm/README.md`, and `assets/spark.skill.md`** to reflect the new division.

Breaking change: users with digests in `~/.config/spark/ingest/` will need to regenerate. One-shot migration script is a nice-to-have but not required — digests are regenerated cheaply.

---

## Lessons learned

1. **When two tools of yours overlap, one must be the source of truth.** We built spark's catalog before TRS had one. Once TRS matured, we didn't collapse the overlap — we ended up with two parallel registries that don't see each other. The user has to remember which one has what.

2. **Write thin wrappers, not parallel implementations.** The moment you reimplement storage, cache invalidation, or listing for something another tool already handles, you've duplicated the work and now have to keep both in sync forever.

3. **Bidirectional awareness is fine if the direction of trust is clear.** TRS knowing about spark (for clone path resolution) is useful. SPARK knowing about TRS (for generation) is useful. But both sides having independent storage of the same artifact is a failure mode.

4. **Passthrough flags are a code smell.** If you forward every flag verbatim to another tool, you're a shim. Either add value per flag or delete the wrapper and let users call the underlying tool directly.

5. **Fleet vs instance is the right abstraction boundary.** The valid spark-level primitive (`--all`) operates on the fleet. The invalid ones (`--budget` on one repo) operate on an instance — that's TRS's job.
