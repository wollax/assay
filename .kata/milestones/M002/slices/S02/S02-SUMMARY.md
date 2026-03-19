---
id: S02
parent: M002
milestone: M002
provides:
  - "run_orchestrated() — DAG-driven parallel executor with std::thread::scope, bounded concurrency, failure propagation, and state persistence"
  - "setup_session() + execute_session() — two-phase pipeline split enabling serialized worktree creation with parallel agent execution"
  - "SetupResult struct for handoff between setup and execute phases"
  - "SessionOutcome, OrchestratorConfig, OrchestratorResult types in assay-core"
  - "SessionRunState, FailurePolicy, OrchestratorPhase, SessionStatus, OrchestratorStatus serializable types in assay-types"
  - "State persistence to .assay/orchestrator/<run_id>/state.json"
requires:
  - slice: S01
    provides: "DependencyGraph with ready_set(), mark_skipped_dependents(), topological_groups()"
affects:
  - S03
  - S05
  - S06
key_files:
  - crates/assay-core/src/orchestrate/executor.rs
  - crates/assay-core/src/pipeline.rs
  - crates/assay-types/src/orchestrate.rs
key_decisions:
  - "D031: Two-phase pipeline split (setup_session + execute_session) for worktree serialization"
  - "D032: Session runner as closure parameter for testability"
  - "D033: Orchestrate feature gate on assay-types"
  - "D034: Generic F: Fn + Sync for session runner instead of dyn trait object"
  - "D035: HarnessWriter excluded from run_orchestrated() signature — caller captures in closure"
patterns_established:
  - "Condvar-based dispatch loop: outer loop acquires batch via ready_set(), spawns scoped threads, workers notify_all on completion"
  - "Event log pattern (Arc<Mutex<Vec<(String, String)>>>) for ordering tests across threads"
  - "AtomicUsize peak concurrency tracking for bounded concurrency proofs"
  - "Static Send assertion (const _: () = { ... }) to enforce thread-safety at compile time"
observability_surfaces:
  - ".assay/orchestrator/<run_id>/state.json — OrchestratorStatus JSON persisted after each session resolves"
  - "OrchestratorPhase in final state: Completed / PartialFailure / Aborted"
  - "SessionStatus.error carries failure message; SessionStatus.skip_reason carries upstream failure reference"
drill_down_paths:
  - .kata/milestones/M002/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M002/slices/S02/tasks/T02-SUMMARY.md
  - .kata/milestones/M002/slices/S02/tasks/T03-SUMMARY.md
  - .kata/milestones/M002/slices/S02/tasks/T04-SUMMARY.md
duration: 65m
verification_result: passed
completed_at: 2026-03-17
---

# S02: Parallel Session Executor

**DAG-driven parallel session executor with bounded concurrency, failure propagation, and atomic state persistence — proven by 18 tests covering diamond DAGs, abort policy, panic recovery, and bounded concurrency.**

## What Happened

Built the complete parallel session executor in four tasks:

**T01 (12m):** Created all serializable orchestrator types — 5 in assay-types (`SessionRunState`, `FailurePolicy`, `OrchestratorPhase`, `SessionStatus`, `OrchestratorStatus`) and 3 in assay-core (`SessionOutcome`, `OrchestratorConfig`, `OrchestratorResult`). All types have full serde derives, `deny_unknown_fields`, inventory registration, and locked schema snapshots. Feature-gated behind `orchestrate` in both crates.

**T02 (8m):** Refactored `run_session()` into `setup_session()` (stages 1-2: spec load + worktree create) and `execute_session()` (stages 3-6: harness + agent + gate + merge check), connected by `SetupResult`. This enables the executor to serialize worktree creation under a mutex while parallelizing agent execution. `run_session()` preserved as composition of both. Zero test changes needed — pure refactor.

**T03 (30m):** Implemented `run_orchestrated()` (~300 lines) — the core dispatch loop using `std::thread::scope` with `Mutex + Condvar`. Builds `DependencyGraph`, generates ULID run_id, enters dispatch loop that computes ready set, spawns bounded worker threads, records outcomes, propagates failures via `mark_skipped_dependents()`, and persists `OrchestratorStatus` to `.assay/orchestrator/<run_id>/state.json` after each resolution. Worker threads wrapped in `catch_unwind` for panic safety. Supports `FailurePolicy::Abort`.

**T04 (15m):** Added 9 comprehensive tests (18 total): diamond DAG ordering, bounded concurrency enforcement via `AtomicUsize` peak tracking, abort policy stops dispatch, panic caught as failure, state.json field validation, single-session compat, all-fail resilience, mixed deps with independent parallelism, 8-session default concurrency.

## Verification

- `cargo test -p assay-types --features orchestrate -- orchestrate` — 9 type round-trip + schema tests pass ✅
- `cargo test -p assay-core --features orchestrate -- orchestrate::executor` — 18 executor tests pass ✅
- `cargo test -p assay-core --features orchestrate` — 718 tests pass ✅
- `cargo test -p assay-core` (without feature) — 671 existing tests pass, executor module absent ✅
- `just ready` — full suite green (fmt, lint, test, deny) ✅
- State.json readability verified by 5 tests (three_independent, state_persistence_has_correct_fields, abort_policy, panic_recovery, single_session) ✅
- Failed session dependents skipped verified by 3 tests (failure_skips_dependents, panic_recovery, all_sessions_fail) ✅

## Requirements Advanced

- R020 (Multi-agent orchestration) — DAG-driven parallel executor with dependency ordering, failure propagation, and bounded concurrency now proven by 18 tests. Remaining: S06 end-to-end integration with real CLI routing.

## Requirements Validated

- None moved to validated this slice (R020 awaits S06 capstone integration)

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

- **Session runner as generic instead of `dyn` type alias**: Changed to `F: Fn(...) + Sync` because `dyn Fn` requires `'static` bounds blocking test closures. Equivalent API, zero-cost. Captured as D034.
- **HarnessWriter removed from `run_orchestrated()` signature**: `HarnessWriter = dyn Fn(...)` is not `Sync`. Callers capture it in the runner closure instead. Captured as D035.
- **Worktree mutex defined but not actively exercised**: Created for serializing `setup_session()` calls, but mock runners don't use the two-phase split. Will be exercised by real session runner in S06.

## Known Limitations

- Worktree mutex exists but is not exercised by current mock-based tests — real two-phase split integration deferred to S06 capstone
- `SessionOutcome::Completed.result` uses `Box<PipelineResult>` due to clippy `large_enum_variant` lint (~400 bytes) — no functional impact
- Pre-existing flaky test `assay-mcp::server::tests::gate_run_nonexistent_working_dir_returns_error` occasionally fails under parallel test execution (unrelated)

## Follow-ups

- S03 consumes `OrchestratorResult` with `SessionOutcome::Completed` entries for merge sequencing
- S05 consumes session list and dependency context for multi-agent prompt generation
- S06 wires `run_orchestrated()` to the real CLI entrypoint and MCP tools

## Files Created/Modified

- `crates/assay-types/src/orchestrate.rs` — new: 5 serializable types with schema registry and 9 unit tests
- `crates/assay-types/src/lib.rs` — feature-gated module declaration and re-exports
- `crates/assay-types/Cargo.toml` — added `orchestrate` feature
- `crates/assay-core/src/orchestrate/executor.rs` — new: `run_orchestrated()` implementation with 18 tests
- `crates/assay-core/src/orchestrate/mod.rs` — added `pub mod executor`
- `crates/assay-core/src/pipeline.rs` — `SetupResult`, `setup_session()`, `execute_session()` public functions
- `crates/assay-core/Cargo.toml` — orchestrate feature forwards to assay-types/orchestrate
- `crates/assay-mcp/Cargo.toml` — enabled orchestrate feature on assay-types dependency
- `crates/assay-types/tests/schema_snapshots.rs` — 5 new feature-gated snapshot tests
- `crates/assay-types/tests/snapshots/` — 5 new schema snapshot files

## Forward Intelligence

### What the next slice should know
- `run_orchestrated()` returns `OrchestratorResult` with a `Vec<(usize, SessionOutcome)>` indexed by session position in the manifest. S03's merge runner should filter for `SessionOutcome::Completed` entries and extract `branch_name` and `changed_files` from the inner `PipelineResult`.
- The function is generic over the session runner — S06 will need to compose `setup_session()` under the worktree mutex + `execute_session()` in parallel as the real production runner.
- `OrchestratorStatus` is persisted as JSON and can be deserialized for status queries — S06's `orchestrate_status` MCP tool reads this file.

### What's fragile
- The condvar dispatch loop has a spurious wakeup guard (`inner loop`) that's critical — removing it causes busy-wait. Test coverage verifies correct behavior but the pattern is subtle.
- `ready_set()` requires `completed ∪ failed` in its `completed` parameter — passing only `completed` causes failed sessions to be re-dispatched. This is a semantic subtlety in the DependencyGraph API.

### Authoritative diagnostics
- `.assay/orchestrator/<run_id>/state.json` — the single source of truth for orchestration state. Deserialize as `OrchestratorStatus` to inspect run progress, per-session state, timing, and error messages.
- `OrchestratorPhase` in the final state distinguishes `Completed` (all pass) from `PartialFailure` (any failed/skipped) from `Aborted` (abort policy triggered).

### What assumptions changed
- Assumed `dyn Fn` would work for session runner — actually needs generic `F: Fn + Sync` due to `'static` lifetime requirement on trait objects (D034)
- Assumed `HarnessWriter` could be passed directly — actually not `Sync`, must be captured in closure (D035)
