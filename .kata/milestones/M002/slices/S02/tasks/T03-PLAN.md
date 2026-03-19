---
estimated_steps: 5
estimated_files: 3
---

# T03: Implement executor dispatch loop with state persistence

**Slice:** S02 — Parallel Session Executor
**Milestone:** M002

## Description

Implement `run_orchestrated()` — the core DAG-driven parallel executor. This function builds a `DependencyGraph`, enters `std::thread::scope` with a condvar-based dispatch loop, serializes worktree creation via a mutex, parallelizes agent execution, propagates failures to dependents, persists state after each completion, and returns `OrchestratorResult`. Worker thread bodies are wrapped in `catch_unwind` for panic safety. This is the primary deliverable of S02.

## Steps

1. Define internal `ExecutorState` struct (behind `cfg(test)` boundary where needed): `completed: HashSet<usize>`, `in_flight: HashSet<usize>`, `skipped: HashSet<usize>`, `failed: HashSet<usize>`, `outcomes: Vec<(String, SessionOutcome)>`, `session_statuses: Vec<SessionStatus>`, `aborted: bool`. Define the session runner type alias: `type SessionRunner = dyn Fn(&ManifestSession, &PipelineConfig, &HarnessWriter) -> Result<PipelineResult, PipelineError> + Sync;`
2. Implement `run_orchestrated()` signature: takes `&RunManifest`, `OrchestratorConfig`, `&HarnessWriter`, `&SessionRunner` → `Result<OrchestratorResult, AssayError>`. Build `DependencyGraph::from_manifest()`. Generate run_id via `ulid::Ulid::new()`. Create `.assay/orchestrator/<run_id>/` directory. Initialize `ExecutorState` and `OrchestratorStatus` with all sessions in `Pending`.
3. Implement the dispatch loop inside `std::thread::scope`: acquire shared state lock → check `all_resolved` (completed + failed + skipped == session_count) → call `ready_set()` → compute `available_slots = max_concurrency - in_flight.len()` → take batch from ready → mark in_flight → for each, spawn scoped thread. On empty batch with in_flight > 0, `condvar.wait()`. Re-check after wake (spurious wakeup guard).
4. Implement worker thread body: wrap in `catch_unwind`. Call `setup_session()` under worktree `Mutex<()>` lock (acquire → call → drop guard). Then call `execute_session()` without the lock. Record start/completion timestamps. On completion: lock shared state → move from in_flight to completed → record `SessionOutcome::Completed`. On failure: lock → remove from in_flight → insert into failed → call `mark_skipped_dependents()` → record `SessionOutcome::Failed` and `SessionOutcome::Skipped` for each dependent. On panic: same as failure with panic message. If `FailurePolicy::Abort`, set `aborted = true`. Persist `OrchestratorStatus` snapshot to `state.json` via atomic tempfile-rename. Call `condvar.notify_all()`.
5. After scope exits: construct `OrchestratorResult` from `ExecutorState`. Set final `OrchestratorPhase` (Completed if all succeeded, PartialFailure if any failed/skipped, Aborted if abort triggered). Persist final state snapshot. Return result. Add 3 basic unit tests using mock session runners: (a) 3 independent sessions all complete, (b) linear chain A→B→C executes in order, (c) A fails → B (depends on A) is skipped, C (independent) completes.

## Must-Haves

- [ ] `run_orchestrated()` is public and feature-gated behind `orchestrate`
- [ ] Dispatch loop uses `Mutex + Condvar` pattern (not busy-wait)
- [ ] Worktree creation serialized via `Mutex<()>` around `setup_session()` calls
- [ ] Agent execution (`execute_session()`) runs outside the worktree mutex
- [ ] `catch_unwind` wraps each worker thread body
- [ ] `mark_skipped_dependents()` called on failure before next `ready_set()`
- [ ] `FailurePolicy::Abort` sets aborted flag, preventing new dispatches
- [ ] State persisted to `.assay/orchestrator/<run_id>/state.json` after each session resolves
- [ ] Atomic write (tempfile + rename) for state persistence
- [ ] 3 basic unit tests pass with mock session runners

## Verification

- `cargo test -p assay-core --features orchestrate -- orchestrate::executor` — 3+ tests pass
- `cargo test -p assay-core --features orchestrate` — all existing tests still pass
- `cargo check -p assay-core --features orchestrate` — no warnings

## Observability Impact

- Signals added/changed: `OrchestratorStatus` JSON written to disk after each session resolution — carries phase, per-session state, timing, error messages
- How a future agent inspects this: Read `.assay/orchestrator/<run_id>/state.json` and deserialize as `OrchestratorStatus`; latest run_id can be found by listing directory sorted by ULID
- Failure state exposed: `SessionOutcome::Failed` preserves `PipelineStage` + message; `SessionOutcome::Skipped` preserves reason with failed upstream name; panic messages captured as failure

## Inputs

- T01 output: `SessionOutcome`, `OrchestratorConfig`, `OrchestratorResult` in executor.rs; `OrchestratorStatus`, `SessionStatus`, `SessionRunState` in assay-types
- T02 output: `setup_session()`, `execute_session()`, `SetupResult` in pipeline.rs
- S01: `DependencyGraph` with `ready_set()`, `mark_skipped_dependents()`, `from_manifest()`
- S01 forward intelligence: `mark_skipped_dependents()` does NOT insert failed_idx; skipped deps count as satisfied in `ready_set()`
- `work_session.rs`: `save_session()` atomic write pattern (tempfile + rename)

## Expected Output

- `crates/assay-core/src/orchestrate/executor.rs` — complete `run_orchestrated()` implementation (~200 lines) with `ExecutorState`, session runner type alias, state persistence, and 3 unit tests
- `crates/assay-core/src/orchestrate/mod.rs` — `pub mod executor` already added in T01
- `.assay/orchestrator/` directory created at runtime during test execution
