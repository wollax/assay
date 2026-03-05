# Phase 15: Run History CLI — Context

## Requirements (from ROADMAP.md)

- HIST-02: User can view recent gate run history for a spec via `assay history <spec>`
- HIST-03: Run history has a configurable retention policy (per-spec file count limit) enforced on save

## Decisions

### 1. History Table Layout & Content

**Command:** `assay gate history <spec>`

**Columns (in order):** `#`, `Timestamp`, `Status` (pass/fail), `Passed`, `Failed`, `Skipped`, `Req Failed`, `Adv Failed`, `Duration`

**Timestamp display:**
- Compact relative for recent: `2m`, `4h` style
- Absolute beyond 1 day: `2026-03-05 14:30`

**Flags:**
- `--json` — output as JSON array (same pattern as `gate run --json`)
- `--limit N` — control how many entries to show; default is 20

**Empty state:** "No history for <spec>" message, exit 0.

### 2. Command Structure

**Top-level:** `assay gate history <spec>` (nested under `gate`, not top-level)

**Subcommands:**
- `assay gate history <spec>` — table of recent runs (default last 20)
- `assay gate history <spec> <run-id>` — formatted detail view of a single run, optimized for agent consumption
- `assay gate history <spec> --last` — formatted detail view of the most recent run

**Flag composition:** `--last` + `--json` composes — dumps the full GateRunRecord JSON for the most recent run.

**Error handling:** Same pattern as `gate run` — "Error: spec 'foo' not found in specs/" and exit 1.

### 3. Retention Configuration & Pruning

**Config location:** Extend existing `[gates]` section in `.assay/config.toml` with a new field:
```toml
[gates]
max_history = 1000
```

**Default:** 1000 (when field is absent or not configured).

**Special value:** `max_history = 0` means unlimited / archive mode (no pruning).

**Pruning trigger:** On every `save()` call in the history module.

**Pruning output:**
- Default: simple stderr message, e.g., "Pruned 3 old run(s) for auth-flow"
- Verbose flag (TBD — could reuse `--verbose` or a dedicated flag): lists pruned filenames
- Suppressed entirely when `--json` is active (no stderr pollution)

## Deferred Ideas

None identified during discussion.

## Existing Code to Extend

- **CLI:** `crates/assay-cli/src/main.rs` — add `History` variant to `GateCommand` enum
- **History module:** `crates/assay-core/src/history/mod.rs` — add `prune()` function, integrate into `save()`
- **Config type:** `crates/assay-types/src/lib.rs` — add `max_history` field to `GatesConfig`
- **Dependencies:** `chrono` already in workspace (used by history module) for relative timestamp formatting
