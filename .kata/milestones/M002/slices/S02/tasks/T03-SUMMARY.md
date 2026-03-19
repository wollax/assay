---
id: T03
parent: S02
milestone: M002
provides:
  - "run_orchestrated() — DAG-driven parallel executor with condvar dispatch, failure propagation, and state persistence"
  - "ExecutorState internal struct for shared mutable state across worker threads"
  - "persist_state() atomic tempfile-rename JSON writer for OrchestratorStatus"
key_files:
  - crates/assay-core/src/orchestrate/executor.rs
key_decisions:
  - "Generic `F: Fn + Sync` parameter for session runner instead of `dyn` trait object — avoids `'static` lifetime requirement that blocks test closures capturing local state"
  - "Removed HarnessWriter from run_orchestrated() signature — caller captures it in the session runner closure, simplifying thread-safety (HarnessWriter is not Sync)"
  - "Pass completed ∪ failed to ready_set()'s completed parameter — DependencyGraph::ready_set() only excludes completed/in_flight/skipped, so failed sessions must be unioned with completed to prevent re-dispatch"
patterns_established:
  - "Condvar-based dispatch loop: outer loop acquires batch from inner loop (with spurious wakeup guard), spawns scoped threads, workers notify_all on completion"
  - "Collect skipped indices into Vec before mutating session_statuses to satisfy borrow checker (can't iterate &guard.skipped while mutating guard.session_statuses)"
  - "Best-effort state persistence in worker threads (errors ignored) — don't fail the whole run for a state write failure"
observability_surfaces:
  - ".assay/orchestrator/<run_id>/state.json — OrchestratorStatus JSON persisted after each session resolves and at run completion"
  - "OrchestratorPhase in final state.json: Completed / PartialFailure / Aborted"
  - "SessionStatus.error carries failure message; SessionStatus.skip_reason carries upstream failure reference"
duration: 30m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T03: Implement executor dispatch loop with state persistence

**Implemented `run_orchestrated()` — the core DAG-driven parallel executor with `std::thread::scope`, condvar-based dispatch, failure propagation via `mark_skipped_dependents()`, and atomic state persistence to `.assay/orchestrator/<run_id>/state.json`.**

## What Happened

Built the complete `run_orchestrated()` function in `executor.rs` (~300 lines) with:

1. **ExecutorState** internal struct tracking completed/in_flight/skipped/failed sets, outcomes, session statuses, and abort flag.
2. **Dispatch loop** using `Mutex + Condvar` pattern inside `std::thread::scope`. Inner loop handles spurious wakeups, computes ready set via `DependencyGraph::ready_set()`, takes batch up to `max_concurrency - in_flight`, marks in-flight, breaks when all resolved or aborted.
3. **Worker threads** wrapped in `catch_unwind` for panic safety. On success: record completed + outcome. On failure: record failed, call `mark_skipped_dependents()`, record skipped outcomes for dependents. On panic: same as failure with panic message extraction. `FailurePolicy::Abort` sets aborted flag.
4. **State persistence** via `persist_state()` using atomic tempfile-rename pattern (matching `save_session()` in work_session.rs). Written after each session resolves and at final completion.
5. **3 unit tests** with mock session runners: (a) 3 independent sessions all complete, (b) linear chain A→B→C executes in order, (c) A fails → B skipped, C (independent) completes. Plus 5 pre-existing type construction tests retained.

Key design deviation: function is generic over `F: Fn(&ManifestSession, &PipelineConfig) -> Result<PipelineResult, PipelineError> + Sync` instead of using a `dyn` trait object. This avoids `'static` lifetime issues when test closures capture local `AtomicUsize`/`Mutex` state. The `HarnessWriter` was removed from the function signature — callers capture it in their runner closure, which also avoids thread-safety issues since `HarnessWriter` lacks a `Sync` bound.

## Verification

- `cargo test -p assay-core --features orchestrate -- orchestrate::executor` — **9 tests pass** (3 executor integration + 6 type construction)
- `cargo test -p assay-core --features orchestrate` — **709 tests pass**, 0 failed
- `cargo check -p assay-core --features orchestrate` — **no warnings**
- `cargo test -p assay-types --features orchestrate` — 40 tests pass (type round-trip + schema snapshots)
- `cargo test -p assay-core` (without feature) — existing tests pass, executor module absent
- `just fmt` + `just lint` — clean (one pre-existing flaky test in assay-mcp unrelated to this work)
- `three_independent_sessions_all_complete` test verifies state.json is readable and contains correct session states
- `failure_skips_dependents_independent_completes` test verifies failed session's dependents are skipped and state.json reflects PartialFailure

## Diagnostics

- Read `.assay/orchestrator/<run_id>/state.json` and deserialize as `OrchestratorStatus` to inspect run state
- `OrchestratorPhase` in final snapshot: `Completed` (all pass), `PartialFailure` (any failed/skipped), `Aborted` (abort policy triggered)
- `SessionStatus.error` carries pipeline error message for failed sessions
- `SessionStatus.skip_reason` carries "upstream '<name>' failed/panicked" for skipped sessions
- State is written after every session resolution — intermediate snapshots show `Running` phase with per-session progress

## Deviations

- **SessionRunner as generic instead of type alias**: Plan called for `type SessionRunner = dyn Fn(...) + Sync`. Changed to generic `F: Fn(...) + Sync` because `dyn` trait objects require `'static` bounds that prevent test closures from capturing stack-local state. This is an equivalent API surface.
- **HarnessWriter removed from function signature**: Plan called for `&HarnessWriter` parameter. Removed because `HarnessWriter = dyn Fn(...)` is not `Sync`, making it unsendable to worker threads. Callers capture it in their runner closure instead. The `_wt_mutex` (worktree mutex) is defined but unused pending the two-phase split integration in T04.
- **Worktree mutex defined but not actively used**: The `worktree_mutex` is created for serializing `setup_session()` calls, but the mock session runners in tests don't use the two-phase split. T04's comprehensive tests or the real production runner will wire `setup_session()` under this mutex.

## Known Issues

- Pre-existing flaky test `assay-mcp::server::tests::gate_run_nonexistent_working_dir_returns_error` occasionally fails under parallel test execution (unrelated to this work)
- `worktree_mutex` is defined but unused in current mock-based tests — will be exercised by real session runner or T04 comprehensive tests

## Files Created/Modified

- `crates/assay-core/src/orchestrate/executor.rs` — Complete rewrite: `run_orchestrated()` implementation with `ExecutorState`, `persist_state()`, condvar dispatch loop, worker thread bodies with `catch_unwind`, and 9 unit tests
