---
id: T03
parent: S04
milestone: M003
provides:
  - "`smelt list` command: enumerates all per-job runs in `.smelt/runs/`, printing a header + one row per job with name, phase, elapsed, and PR URL"
  - "Graceful degradation: missing `.smelt/runs/` dir prints 'No past runs.' (exit 0); corrupt or missing state.toml files warn to stderr and are skipped"
  - "`ensure_gitignore_assay()` helper in run.rs: creates or appends `.assay/` to repo `.gitignore` before container work begins"
  - "Idempotent gitignore guard: calling twice does not duplicate `.assay/`; correctly handles files with or without trailing newline"
  - "`ensure_gitignore_assay()` wired into `run_with_cancellation()` after Phase 3 runtime check (best-effort: failure is only a warning)"
key_files:
  - crates/smelt-cli/src/commands/list.rs
  - crates/smelt-cli/src/commands/mod.rs
  - crates/smelt-cli/src/main.rs
  - crates/smelt-cli/src/commands/run.rs
key_decisions:
  - "ensure_gitignore_assay() failure is non-fatal (warn + continue): repo .gitignore is a quality-of-life guard, not a correctness requirement for job execution"
  - "smelt list is strictly forward-looking — reads only per-job state.toml files, never legacy run-state.toml"
  - "Entry without state.toml is silently skipped (not warned); corrupt state.toml is warned to stderr — distinguish 'no state yet' from 'corrupted state'"
patterns_established:
  - "Aggregate inspection surface: `smelt list` is the correct tool for 'what runs exist in this repo?'; `smelt status <job>` is for single-job detail"
  - "Best-effort gitignore guard pattern: wrap in `if let Ok(...)` + `eprintln!([WARN])` to avoid blocking job execution on a non-critical side-effect"
observability_surfaces:
  - "`smelt list` — aggregate view of all `.smelt/runs/<job>/state.toml` files; shows job name, phase, elapsed seconds, PR URL"
  - "Corrupt state.toml: `[WARN] skipping <path>: <error>` to stderr — a future agent can locate and remove/re-run the specific failing job"
duration: 15min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T03: Add `smelt list` command and `.assay/` gitignore guard

**`smelt list` enumerates `.smelt/runs/` per-job state files with tabular output; `ensure_gitignore_assay()` idempotently adds `.assay/` to repo `.gitignore` before container provisioning.**

## What Happened

Created `crates/smelt-cli/src/commands/list.rs` with `ListArgs` (single `--dir` flag defaulting to `.`) and `execute()`. The function walks `.smelt/runs/`, reads `state.toml` per-job directory via `JobMonitor::read()`, and prints a fixed-width table (JOB / PHASE / ELAPSED / PR URL). Entries without `state.toml` are silently skipped; entries with corrupt TOML warn to stderr and continue. If no valid entries exist (including missing `runs/` dir), prints "No past runs." and returns Ok(0).

Added `ensure_gitignore_assay()` to `run.rs` as a private helper. It resolves `.gitignore` in the repo path, checks for existing `.assay/` content (idempotency), and either creates the file or appends with proper newline boundary handling. Called in `run_with_cancellation()` after Phase 3 (runtime check) as a best-effort non-blocking operation — failure emits `[WARN]` to stderr but does not abort the run.

Registered `smelt list` in `commands/mod.rs` and `main.rs` following the same pattern as `smelt init` (T02).

## Verification

- `cargo test -p smelt-cli` — all 8 new tests pass (4 list + 4 gitignore), no regressions in existing 59 tests
- `test_list_skips_corrupt_state` — corrupt TOML skipped cleanly, exit 0
- `test_ensure_gitignore_trailing_newline` — verified resulting file contains no `target/.assay/` mash; entries on separate lines
- `test_ensure_gitignore_idempotent` — confirmed `.assay/` appears exactly once after two calls
- Pre-existing `test_cli_run_invalid_manifest` failure confirmed as pre-existing (reproduced on baseline before any changes)

## Diagnostics

- `smelt list` is the primary aggregate inspection surface for all runs in a repo directory
- Corrupt state files surface their path in `[WARN] skipping <path>: <error>` to stderr
- `smelt list --dir <path>` can be pointed at any directory containing a `.smelt/runs/` tree

## Deviations

The task plan listed 3+ gitignore tests but specified 4 (creates, appends, trailing-newline, idempotent). Implemented all 4. The "3+" phrasing in the must-haves was a floor, not a ceiling.

## Known Issues

`test_cli_run_invalid_manifest` in `tests/docker_lifecycle.rs` was failing before this task (pre-existing). Unrelated to T03 changes.

## Files Created/Modified

- `crates/smelt-cli/src/commands/list.rs` — new file: `ListArgs`, `execute()`, 4 unit tests
- `crates/smelt-cli/src/commands/mod.rs` — added `pub mod list;`
- `crates/smelt-cli/src/main.rs` — added `List` variant and match arm
- `crates/smelt-cli/src/commands/run.rs` — added `ensure_gitignore_assay()` helper, call site after Phase 3, 4 unit tests
