---
id: S04
parent: M003
milestone: M003
provides:
  - "Per-job state isolation: JobMonitor writes/reads state.toml under .smelt/runs/<job-name>/; read_legacy() reads legacy flat run-state.toml"
  - "smelt run writes state to .smelt/runs/<manifest.job.name>/state.toml; watch and status read from per-job path"
  - "smelt status <job-name> reads per-job state; smelt status (no args) falls back to legacy .smelt/run-state.toml via read_legacy()"
  - "smelt init command: writes commented skeleton job-manifest.toml; passes validate(); idempotency guard exits 1 if file exists"
  - "smelt list command: enumerates .smelt/runs/ per-job entries; tabular output (JOB/PHASE/ELAPSED/PR URL); gracefully skips corrupt/missing state.toml"
  - ".assay/ gitignore guard: ensure_gitignore_assay() creates or appends .assay/ to repo .gitignore before Phase 5; idempotent; non-fatal on error"
requires: []
affects:
  - S05
  - S06
key_files:
  - crates/smelt-core/src/monitor.rs
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/src/commands/watch.rs
  - crates/smelt-cli/src/commands/status.rs
  - crates/smelt-cli/src/commands/init.rs
  - crates/smelt-cli/src/commands/list.rs
  - crates/smelt-cli/src/commands/mod.rs
  - crates/smelt-cli/src/main.rs
key_decisions:
  - "D064: smelt status backward compat via optional positional job_name — None falls back to read_legacy(), Some reads per-job path"
  - "D065: smelt init skeleton as raw string literal const — toml::to_string_pretty strips comments, raw literal preserves them"
  - "D066: .assay/ gitignore guard placed after Phase 3 (runtime check), before Phase 5 (provision) — host-side I/O with no container dependency; non-fatal"
  - "state file renamed run-state.toml → state.toml; per-job canonical path is .smelt/runs/<job-name>/state.toml (D034 superseded)"
  - "read_legacy() is a static method on JobMonitor — backward-compat reads always go through an explicitly named method, never through read()"
  - "CWD_LOCK mutex added to init tests — set_current_dir() is process-global; serialize parallel test threads"
  - "ensure_gitignore_assay() failure is non-fatal (warn + continue) — gitignore is a quality-of-life guard, not a correctness requirement"
  - "smelt list is strictly forward-looking — reads only per-job state.toml files, never legacy run-state.toml"
patterns_established:
  - "per-job state isolation: .smelt/runs/<job-name>/state.toml is canonical state path from S04 onward"
  - "read_legacy() pattern: backward-compat reads always go through an explicitly named method"
  - "idempotency guard pattern: check file existence, print actionable error to stderr, return Ok(1)"
  - "SKELETON as raw string literal: struct serialization not used for templated output that must preserve comments"
  - "aggregate inspection surface: smelt list for all runs; smelt status <job> for single-job detail"
  - "best-effort gitignore guard: wrap in if let Ok(...) + eprintln!([WARN]) to avoid blocking job execution on non-critical side-effects"
observability_surfaces:
  - "smelt status <job-name> — reads .smelt/runs/<job-name>/state.toml (per-job)"
  - "smelt status (no args) — reads legacy .smelt/run-state.toml via read_legacy()"
  - "smelt list — aggregate view of all .smelt/runs/<job>/state.toml files; shows job name, phase, elapsed, PR URL"
  - "smelt watch <job-name> error message includes expected state_dir path for diagnosis when no PR was created"
  - "smelt list corrupt state: [WARN] skipping <path>: <error> to stderr for actionable diagnosis"
drill_down_paths:
  - .kata/milestones/M003/slices/S04/tasks/T01-SUMMARY.md
  - .kata/milestones/M003/slices/S04/tasks/T02-SUMMARY.md
  - .kata/milestones/M003/slices/S04/tasks/T03-SUMMARY.md
duration: 50min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
---

# S04: Infrastructure Hardening

**Per-job state isolation, `smelt init`, `smelt list`, and `.assay/` gitignore guard — all verified by `cargo test`; no pre-existing test regressions introduced.**

## What Happened

Three independent deliverables landed across three tasks.

**T01 — Per-job state isolation:** Renamed `run-state.toml` → `state.toml` across `JobMonitor::write()`, `read()`, and `cleanup()`. Added `JobMonitor::read_legacy(base_dir)` as the sole entry point for the legacy flat path — backward compat is explicitly named, not implicit. Updated `run.rs` to compute `state_dir = .smelt/runs/<manifest.job.name>` (one-line change after manifest load); `create_dir_all` in `write()` creates the nested directory automatically. Fixed `watch.rs` `execute()` to use `.smelt/runs/<args.job_name>` and updated `persist_run_state()` to write `state.toml`. Gave `StatusArgs` an optional positional `job_name` field — when `Some`, reads per-job path; when `None`, calls `read_legacy()` for backward compat. Added 3 new `monitor.rs` unit tests and 1 new `status.rs` test.

**T02 — `smelt init`:** Created `init.rs` with a `SKELETON` raw-string-literal containing a fully-commented TOML manifest (all serde-required fields, inline `#`-comments per section). `execute()` checks file existence first — if present, prints clear error to stderr and returns `Ok(1)`. If absent, writes the skeleton and prints the next command. Skeleton passes `JobManifest::validate()` without modification. Added `CWD_LOCK: Mutex<()>` to serialize parallel `set_current_dir()` calls in tests (process-global mutation). Registered in `mod.rs` and `main.rs` Commands enum.

**T03 — `smelt list` + `.assay/` gitignore guard:** Created `list.rs` walking `.smelt/runs/` — reads `state.toml` per entry, silently skips entries without state.toml, warns to stderr on corrupt TOML, prints a fixed-width table (JOB / PHASE / ELAPSED / PR URL), falls back to "No past runs." when directory is absent or empty. Added `ensure_gitignore_assay()` to `run.rs`: resolves `.gitignore` in repo path, checks for existing `.assay/` (idempotency), creates or appends with correct newline boundary handling. Wired as a best-effort call in `run_with_cancellation()` after Phase 3 — failure emits `[WARN]` but does not abort the run. Registered `smelt list` in `mod.rs` and `main.rs`.

## Verification

- `cargo test -p smelt-core`: 121 passed, 0 failed (includes all monitor state-path tests)
- `cargo test -p smelt-cli` (lib + integration): all pass — 27 lib tests, 5 status_pr tests, 12 dry_run tests, 22 docker_lifecycle tests pass
- Pre-existing failure `test_cli_run_invalid_manifest` (docker_lifecycle) is unrelated to S04; confirmed pre-existing before any S04 changes
- All new tests verified: `test_read_legacy_reads_flat_file`, `test_state_path_resolution`, `test_cleanup_uses_state_toml`, `test_status_legacy_backward_compat`, `test_init_creates_manifest`, `test_init_fails_if_file_exists`, `test_init_skeleton_parses`, `test_list_empty_runs_dir`, `test_list_with_state_files`, `test_list_skips_corrupt_state`, `test_ensure_gitignore_creates`, `test_ensure_gitignore_appends`, `test_ensure_gitignore_idempotent`, `test_ensure_gitignore_trailing_newline`

## Requirements Advanced

- R006 — Per-job state directories implemented: `.smelt/runs/<job-name>/state.toml`; concurrent runs with different job names are isolated; `smelt status <job-name>` reads the correct per-job path; backward-compat `smelt status` (no args) reads legacy flat file. **Now validated.**
- R007 — `smelt init` implemented: creates commented skeleton that passes `validate()`; idempotency guard exits 1 if file already exists; `smelt --help` shows the subcommand. **Now validated.**
- R008 — `.assay/` gitignore guard implemented: `ensure_gitignore_assay()` creates or appends `.assay/` before container provisioning; idempotent; 4 unit tests prove all cases. **Now validated.**

## Requirements Validated

- R006 — Concurrent smelt runs use isolated state directories: per-job paths written and read correctly; `test_state_path_resolution` and `test_read_legacy_reads_flat_file` confirm isolation and backward compat. Status: **validated**.
- R007 — `smelt init` generates a skeleton manifest: `test_init_creates_manifest` loads and validates the generated file; `test_init_fails_if_file_exists` confirms the idempotency guard. Status: **validated**.
- R008 — `.assay/` protected from accidental git commits: `test_ensure_gitignore_creates/appends/idempotent/trailing_newline` cover all cases. Status: **validated**.

## New Requirements Surfaced

- None.

## Requirements Invalidated or Re-scoped

- None.

## Deviations

- T02: Added `CWD_LOCK: Mutex<()>` to serialize `set_current_dir()` calls in init tests — not mentioned in the plan but required for test correctness in parallel test threads.
- T03: Plan specified "3+" gitignore guard tests; implemented exactly 4 (`creates`, `appends`, `idempotent`, `trailing_newline`). The "3+" phrasing was a floor, not a ceiling.

## Known Limitations

- `test_cli_run_invalid_manifest` in `tests/docker_lifecycle.rs` is a pre-existing failure unrelated to S04. It was failing before any S04 changes.
- `smelt list` shows elapsed time in seconds (not human-friendly like "2h 3m"); sufficient for MVP but could be improved in S06.
- `ensure_gitignore_assay()` guards only `.assay/`; it does not add `.smelt/` to `.gitignore`. This was out of scope for S04 — S04 only addressed R008.

## Follow-ups

- S05 should export the new `smelt list` and `smelt init` CLi commands in the `smelt-core` public API surface as appropriate (likely not — they are CLI-only concerns).
- S06 integration proof should include `smelt init → smelt run --dry-run` as a smoke test of the generated skeleton.

## Files Created/Modified

- `crates/smelt-core/src/monitor.rs` — write/read/cleanup use state.toml; added read_legacy(); 3 new unit tests; 2 existing tests updated
- `crates/smelt-cli/src/commands/run.rs` — state_dir uses .smelt/runs/<job.name>; added ensure_gitignore_assay(); call site after Phase 3; 4 unit tests
- `crates/smelt-cli/src/commands/watch.rs` — execute() uses per-job state_dir; persist_run_state writes state.toml; test helper updated
- `crates/smelt-cli/src/commands/status.rs` — optional positional job_name; execute() routes to read() or read_legacy(); test helpers and tests updated
- `crates/smelt-cli/src/commands/init.rs` — new: InitArgs, execute(), SKELETON const, 3 unit tests
- `crates/smelt-cli/src/commands/list.rs` — new: ListArgs, execute(), 4 unit tests
- `crates/smelt-cli/src/commands/mod.rs` — added pub mod init; pub mod list
- `crates/smelt-cli/src/main.rs` — added Init and List variants + match arms

## Forward Intelligence

### What the next slice should know
- `smelt-core`'s public API surface needs `JobMonitor::read_legacy()` documented alongside `JobMonitor::read()` — S05 must add `#![deny(missing_docs)]` and these two entry points have distinct semantics that require doc comments explaining when to use each.
- `smelt init` skeleton template is a raw string literal in `init.rs` — updating the skeleton (e.g. to add a `[forge]` example section) requires editing the `SKELETON` const directly. No automated roundtrip from structs.
- `ensure_gitignore_assay()` is private to `run.rs` — if S05 wants to expose it as a library function, it needs to be promoted to `smelt-core`.

### What's fragile
- `init.rs` tests use `CWD_LOCK` to serialize `set_current_dir()` calls — any new test that uses `set_current_dir()` in the same process must also acquire `CWD_LOCK`, or tests will race. This is a known limitation of process-global working directory mutation.
- `smelt list` walks `.smelt/runs/` using `std::fs::read_dir()` — the directory listing order is OS-dependent, not sorted. Output order is non-deterministic. Fine for human consumption; automated assertions in tests must not depend on listing order.

### Authoritative diagnostics
- `smelt status <job-name>` — single source of truth for per-job state; reads `.smelt/runs/<name>/state.toml`
- `smelt list` — aggregate view of all runs in the working directory
- `[WARN] skipping <path>: <error>` in `smelt list` output — indicates a corrupt state file at a specific path

### What assumptions changed
- D034 assumed a single-job flat state file was sufficient for M001 scope. S04 supersedes it: per-job directories are now the canonical storage model. `read_legacy()` is the bridge; no code outside that method should reference `run-state.toml`.
