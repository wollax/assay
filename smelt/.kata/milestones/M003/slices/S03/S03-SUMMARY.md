---
id: S03
parent: M003
milestone: M003
provides:
  - "RunState with 5 new serde-default fields: pr_status, ci_status, review_count, forge_repo, forge_token_env"
  - "PrState and CiStatus enums gain Serialize/Deserialize (snake_case) — storable in TOML state file"
  - "Phase 9 in run.rs persists forge_repo and forge_token_env into RunState at PR creation time"
  - "format_pr_section(state) -> Option<String> in status.rs — renders PR section when pr_url is set, absent otherwise"
  - "smelt watch <job-name> command: polls GitHubForge every 30s, updates RunState, exits 0 on Merged / 1 on Closed"
  - "run_watch<F: ForgeClient> inner function — testable polling loop accepting a generic ForgeClient"
  - "MockForge under #[cfg(test)] with VecDeque<PrStatus> — pre-programmed test double for polling unit tests"
  - "9 new tests: 5 in tests/status_pr.rs (display/compat) + 4 in watch.rs (exit/state/poll unit tests)"
requires:
  - slice: S01
    provides: "ForgeClient trait, GitHubForge, PrState, CiStatus, PrHandle, PrStatus, ForgeConfig"
  - slice: S02
    provides: "RunState.pr_url, RunState.pr_number — consumed by watch execute() for GitHub API calls"
affects:
  - S05
key_files:
  - crates/smelt-core/src/forge.rs
  - crates/smelt-core/src/monitor.rs
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/src/commands/status.rs
  - crates/smelt-cli/src/commands/watch.rs
  - crates/smelt-cli/src/commands/mod.rs
  - crates/smelt-cli/src/main.rs
  - crates/smelt-cli/tests/status_pr.rs
key_decisions:
  - "D059: forge_repo and forge_token_env stored in RunState so smelt watch is self-contained (no manifest needed at watch time)"
  - "D060: run_watch<F: ForgeClient> generic inner function — avoids dyn trait, enables MockForge injection without object-safety issues"
  - "D061: transient poll errors are non-fatal — print [WARN] and retry; only terminal PR states or user cancellation exit the loop"
  - "D062: toml promoted to regular dep in smelt-cli so persist_run_state() works outside #[cfg(test)]"
  - "D063: PrState/CiStatus gained Serialize/Deserialize derives to be storable in RunState TOML"
patterns_established:
  - "Inner run_watch<F: ForgeClient> generic pattern — same extraction strategy as should_create_pr() in S02; reusable for future forge-backed command tests"
  - "MockForge with VecDeque<PrStatus> + default fallback — pop from queue, fall back to default when empty; pattern is reusable for any ForgeClient-consuming command test"
  - "Duration::ZERO in unit tests — tokio::time::sleep(Duration::ZERO) yields without blocking; keeps watch tests fast without clock mocking"
observability_surfaces:
  - "stderr poll line: [HH:MM:SS] PR #N — state: X | CI: Y | reviews: N — emitted once per interval"
  - "Terminal lines: 'PR merged.' (exit 0) or 'PR closed without merging.' (exit 1) emitted to stderr"
  - "cat .smelt/run-state.toml — pr_status, ci_status, review_count, forge_repo, forge_token_env visible after Phase 9; pr_status/ci_status/review_count updated after each successful watch poll"
  - "[WARN] poll failed: ... — transient API errors printed to stderr without aborting"
  - "smelt status — renders ── Pull Request ── section with URL, state, CI, reviews when pr_url is set"
drill_down_paths:
  - .kata/milestones/M003/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M003/slices/S03/tasks/T02-SUMMARY.md
duration: 45min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
---

# S03: PR Status Tracking

**`smelt status` renders a live PR section; `smelt watch <job-name>` blocks and polls via a MockForge-testable inner function, updating RunState on each iteration and exiting 0 on Merged / 1 on Closed.**

## What Happened

**T01** extended `RunState` in `monitor.rs` with five `#[serde(default)]` fields — `pr_status`, `ci_status`, `review_count`, `forge_repo`, `forge_token_env` — all backward-compatible with existing state files. `PrState` and `CiStatus` in `forge.rs` gained `Serialize`/`Deserialize` derives so they can round-trip through TOML. Phase 9 in `run.rs` was updated to persist `forge_repo` and `forge_token_env` alongside the existing `pr_url`/`pr_number` fields. `status.rs` gained a `format_pr_section()` function (public, not pub(crate), for integration-test access) that returns `None` when no PR exists and `Some(text)` otherwise, with graceful "unknown" fallbacks for uncached fields and "0" for review count when absent.

**T02** created `watch.rs` with `WatchArgs`, a public `execute()` that performs all guard checks (missing state, no `pr_url`, missing `forge_token_env`, unset token, missing `forge_repo`, missing `pr_number`) before constructing `GitHubForge` and delegating to `run_watch`. The inner `run_watch<F: ForgeClient>` is generic over the forge client — the same D060 extraction pattern used for `should_create_pr()` in S02 — accepting a `MockForge` in tests and `GitHubForge` in production. State is persisted after each poll via a best-effort `persist_run_state()` helper; transient errors are warned and swallowed (D061). `toml` was promoted from a dev dependency to a regular dependency (D062) so the serializer works outside test code. The `Watch` subcommand was wired into `Commands` and the main match arm.

## Verification

- `cargo test -p smelt-cli --test status_pr -q` — 5 tests pass (format_pr_section absent/present/fields/unknown/backward-compat)
- `cargo test -p smelt-cli --lib -q` — 15 tests pass, including all 4 new watch unit tests (exits_0_on_merged, exits_1_on_closed, immediate_merged, updates_run_state_each_poll)
- `cargo test --workspace -q` — all tests pass (no regressions across smelt-core, smelt-cli)
- `cargo build --bin smelt` — clean, 0 errors, 0 warnings
- `cargo run --bin smelt -- watch --help` — shows `<JOB_NAME>` positional and `--interval-secs` flag
- `smelt --help` — shows `watch` subcommand in the command list

## Requirements Advanced

- R003 (smelt status shows PR state and CI status) — `format_pr_section()` now renders URL, state, CI status, and review count; section is absent when no PR exists; proven by 5 unit tests in `tests/status_pr.rs`
- R004 (smelt watch blocks until PR merges or closes) — `smelt watch <job-name>` implemented with correct exit codes (0/Merged, 1/Closed), 30s default interval, state persistence, and clear error messages for all guard conditions; proven by 4 unit tests in `watch.rs`

## Requirements Validated

- R003 — validated: `format_pr_section` is unit-tested for all display cases including backward-compat TOML without new fields; behavior is deterministic and fully covered by automated tests
- R004 — validated: `run_watch` exit logic proven via MockForge; guard conditions (no URL, missing token) proven with explicit error-path coverage

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

None — both tasks implemented exactly as specified in the plan.

## Known Limitations

- `smelt watch` currently has no retry limit for transient errors (D061 is intentionally non-fatal); a future improvement could add `--max-retries` or abort-after-N-consecutive-failures behavior
- Live end-to-end proof of `smelt watch` with a real GitHub repo is deferred to S06 UAT; this slice uses mock-only coverage for the polling loop
- `review_count` displays as `0` when `None` (not "unknown") — this reflects the natural expectation for a newly-created PR with no reviews, and is a display decision, not a data gap

## Follow-ups

- S04 will add per-job state directories (`.smelt/runs/<job-name>/state.toml`); `watch.rs`'s `execute()` currently reads from the flat `.smelt/run-state.toml` path — it must be updated in S04 to use the per-job path resolver
- S05 should verify that `ForgeClient`, `PrState`, `CiStatus`, and `PrStatus` are all documented and re-exported cleanly from `smelt_core`
- S06 will perform live end-to-end UAT: `smelt run` → PR created → `smelt watch` → merge → watch exits 0

## Files Created/Modified

- `crates/smelt-core/src/forge.rs` — PrState and CiStatus gained Serialize/Deserialize + serde(rename_all = snake_case)
- `crates/smelt-core/src/monitor.rs` — 5 new #[serde(default)] RunState fields; JobMonitor::new() init updated
- `crates/smelt-cli/src/commands/run.rs` — Phase 9 now persists forge_repo and forge_token_env
- `crates/smelt-cli/src/commands/status.rs` — format_pr_section (pub) added; wired into print_status()
- `crates/smelt-cli/tests/status_pr.rs` — new: 5 unit tests covering all display and backward-compat cases
- `crates/smelt-cli/src/commands/watch.rs` — new: WatchArgs, execute(), run_watch<F>(), persist_run_state(), MockForge, 4 unit tests
- `crates/smelt-cli/src/commands/mod.rs` — added `pub mod watch;`
- `crates/smelt-cli/src/main.rs` — Watch variant and match arm added to Commands
- `crates/smelt-cli/Cargo.toml` — toml promoted from dev-dep to regular dep

## Forward Intelligence

### What the next slice should know
- `watch.rs` `execute()` reads the state file from the flat `.smelt/run-state.toml` path (not a per-job path); S04's state path migration must update this in addition to `status.rs`
- `persist_run_state()` in `watch.rs` is a standalone helper (not via JobMonitor) because `JobMonitor.state_dir` is private; if S04 exposes a `state_path()` accessor or a `write_state()` method on JobMonitor, `persist_run_state` should be consolidated
- `format_pr_section` is `pub` (not `pub(crate)`) to allow access from the `tests/` integration test directory; S05 should decide if this should be re-exported from the library surface or kept CLI-internal

### What's fragile
- `persist_run_state()` silently swallows errors — a write failure during a `smelt watch` session will cause the state file to drift from what the polling loop has observed; `smelt status` would then show stale values until the next successful write
- The guard in `execute()` checks `forge_token_env` presence in RunState but derives the actual token at runtime from the env; if the env var is unset between `smelt run` and `smelt watch`, the watch will fail with a clear error, but the state file will still be intact

### Authoritative diagnostics
- `cat .smelt/run-state.toml` — after `smelt run` Phase 9, check `pr_status`, `ci_status`, `review_count`, `forge_repo`, `forge_token_env` are all populated; after each `smelt watch` poll, `pr_status`/`ci_status`/`review_count` should update
- `smelt status` output — `── Pull Request ──` section is the user-facing rendering of the same fields; if section is absent, `pr_url` is None in the state file
- Stderr from `smelt watch` — each `[HH:MM:SS]` line captures the exact forge state at that poll; the final `PR merged.` or `PR closed without merging.` is the authoritative exit signal

### What assumptions changed
- Original plan assumed `review_count` from D054 (`pr.review_comments` = inline diff comments) — confirmed correct; no change to this field source
- `MockForge` was spec'd with `Vec<PrStatus>` in the plan; implemented with `Mutex<VecDeque<PrStatus>>` to satisfy async Send bounds — functionally identical from the test's perspective
