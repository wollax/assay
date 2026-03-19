---
id: T04
parent: S02
milestone: M002
provides:
  - "18 executor tests covering all must-haves: diamond DAG ordering, bounded concurrency, abort policy, panic recovery, state persistence, single-session compat, all-fail no-deadlock, mixed deps with independent parallelism, 8-session default concurrency"
key_files:
  - crates/assay-core/src/orchestrate/executor.rs
key_decisions:
  - "Arc<Mutex<Vec>> event log pattern for ordering tests — captures start/end events with positional assertions to prove DAG constraints"
  - "AtomicUsize::fetch_max for peak concurrency tracking — lock-free peak measurement without interfering with executor timing"
patterns_established:
  - "Event log pattern: Arc<Mutex<Vec<(String, String)>>> with (name, 'start'/'end') tuples for verifying execution ordering across threads"
  - "Peak concurrency pattern: AtomicUsize current + AtomicUsize peak with fetch_max for bounded concurrency proofs"
observability_surfaces:
  - "Tests serve as executable documentation of executor behavior — no new runtime observability surfaces"
duration: 15m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T04: Comprehensive executor tests and full verification

**Added 9 new executor tests (18 total) proving all S02 must-haves: diamond DAG ordering, bounded concurrency, abort policy, panic recovery, state persistence, single-session compat, and all-fail resilience.**

## What Happened

Added 9 new tests to the executor test module, bringing the total from 9 (T03) to 18:

1. **diamond_dag_d_runs_after_b_and_c** — A→{B,C}→D diamond graph; verifies D starts only after both B and C end using positional event log assertions
2. **bounded_concurrency_enforced** — 5 independent sessions with max_concurrency:2; uses AtomicUsize peak tracking to prove peak ≤ 2 and actually reaches 2
3. **eight_independent_sessions_default_concurrency** — 8 sessions with default config; all complete
4. **abort_policy_stops_dispatch** — 5 independent sessions with FailurePolicy::Abort and max_concurrency:1; only the failing session runs, state.json shows Aborted phase
5. **panic_in_runner_caught_as_failure** — A panics, B depends on A; panic caught as Failed with "panic:" prefix, B skipped, state shows PartialFailure
6. **single_session_through_orchestrator** — solo session works correctly, state.json shows Completed
7. **all_sessions_fail_no_deadlock** — all runners return Err, dependent sessions skipped, no deadlock
8. **state_persistence_has_correct_fields** — mixed pass/fail/skip scenario; deserializes state.json as OrchestratorStatus and validates all fields (run_id, phase, failure_policy, completed_at, per-session state/started_at/completed_at/duration_secs/error/skip_reason)
9. **mixed_deps_and_independent_run_concurrently** — A→B chain + C independent; C runs in parallel with A (doesn't wait for chain)

## Verification

- `cargo test -p assay-core --features orchestrate -- orchestrate::executor` — 18 tests pass ✓
- `cargo test -p assay-core` (without orchestrate feature) — all existing tests pass, executor module absent ✓
- `just ready` — full suite green (fmt, lint, test, deny) ✓

Slice-level verification:
- `cargo test -p assay-types -- orchestrate` — type round-trip and schema tests ✓
- `cargo test -p assay-core --features orchestrate -- orchestrate::executor` — 18 executor unit tests ✓
- `cargo test -p assay-core --features orchestrate` — all core tests pass ✓
- `cargo test -p assay-core` (without feature) — existing tests pass ✓
- `just ready` — full suite green ✓
- State.json readable with correct session states — verified by `three_independent_sessions_all_complete`, `state_persistence_has_correct_fields`, `abort_policy_stops_dispatch`, `panic_in_runner_caught_as_failure`, `single_session_through_orchestrator` ✓
- Failed session dependents skipped — verified by `failure_skips_dependents_independent_completes`, `panic_in_runner_caught_as_failure`, `all_sessions_fail_no_deadlock` ✓

## Diagnostics

Tests serve as executable documentation of executor behavior. No new runtime observability surfaces — tests validate the existing surfaces (state.json persistence, OrchestratorStatus deserialization, SessionOutcome error/skip_reason fields).

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/orchestrate/executor.rs` — added 9 new tests (diamond DAG, bounded concurrency, abort policy, panic recovery, single session, all-fail, state persistence fields, 8-session default, mixed deps+independent)
