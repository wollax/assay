# S02: Parallel Session Executor

**Goal:** `run_orchestrated()` launches independent manifest sessions concurrently via `std::thread::scope` with bounded concurrency, serialized worktree creation, dependency-ordered dispatch, failure propagation, and per-session state persistence.
**Demo:** Unit tests with mock session runners prove correct parallel dispatch, dependency ordering, failure skip propagation, bounded concurrency, and readable state persistence — all behind the `orchestrate` feature gate.

## Must-Haves

- `run_orchestrated()` public function accepting manifest, config, harness writer, and session runner closure
- Bounded concurrency (default: `min(sessions, 8)`, configurable via `OrchestratorConfig`)
- Worktree creation serialized via `Mutex` (D018) while agent execution runs in parallel
- Dependency ordering via `DependencyGraph::ready_set()` from S01
- Failed session dependents skipped via `mark_skipped_dependents()` (D020)
- `FailurePolicy::Abort` stops dispatching new sessions on first failure
- State persisted to `.assay/orchestrator/<run_id>/state.json` after each completion (D022)
- Panic safety via `std::panic::catch_unwind()` in worker threads
- All new code behind `cfg(feature = "orchestrate")` (D002)
- Zero traits — closures only (D001)
- Sync only — `std::thread::scope`, no tokio (D007/D017)

## Proof Level

- This slice proves: contract + integration (mock session runners, not real agents)
- Real runtime required: no (mock session runners simulate pipeline execution)
- Human/UAT required: no (deferred to S06 capstone)

## Verification

- `cargo test -p assay-types -- orchestrate` — type round-trip and schema tests
- `cargo test -p assay-core --features orchestrate -- orchestrate::executor` — executor unit tests
- `cargo test -p assay-core --features orchestrate` — all core tests pass (existing + new)
- `cargo test -p assay-core` (without feature) — existing tests pass, executor module absent
- `just ready` — full suite green (fmt, lint, test, deny)
- At least one test verifies state.json is readable and contains correct session states
- At least one test verifies a failed session's dependents are skipped

## Observability / Diagnostics

- Runtime signals: `OrchestratorStatus` persisted as JSON after each session completion — shows phase, per-session state, timing, and error messages
- Inspection surfaces: `.assay/orchestrator/<run_id>/state.json` readable by `orchestrate_status` MCP tool (S06)
- Failure visibility: `SessionOutcome::Failed` carries `PipelineStage` + message + recovery guidance; `SessionOutcome::Skipped` carries reason referencing the failed upstream; `OrchestratorPhase::PartialFailure` on final status when any session failed
- Redaction constraints: none (no secrets in orchestrator state)

## Integration Closure

- Upstream surfaces consumed: `DependencyGraph` (ready_set, mark_skipped_dependents) from S01; `run_session()` / pipeline types from M001; `save_session()` atomic write pattern from work_session
- New wiring introduced in this slice: `run_orchestrated()` as the core execution engine; two-phase pipeline split (`setup_session` + `execute_session`) enabling serialized worktree creation with parallel agent execution; `OrchestratorStatus` state persistence for status queries
- What remains before the milestone is truly usable end-to-end: S03 (merge runner to merge completed branches), S05 (harness CLI + scope enforcement), S06 (CLI routing + MCP tools + real end-to-end integration)

## Tasks

- [x] **T01: Add orchestrator types to assay-types and assay-core** `est:45m`
  - Why: All downstream code needs the serializable status types (assay-types) and executor result types (assay-core) before the executor can be built
  - Files: `crates/assay-types/src/orchestrate.rs`, `crates/assay-types/src/lib.rs`, `crates/assay-core/src/orchestrate/executor.rs`, `crates/assay-core/src/orchestrate/mod.rs`
  - Do: Create `SessionRunState`, `FailurePolicy`, `SessionStatus`, `OrchestratorStatus`, `OrchestratorPhase` in assay-types with full derives (Serialize, Deserialize, JsonSchema, deny_unknown_fields). Create `SessionOutcome`, `OrchestratorConfig`, `OrchestratorResult` in assay-core executor module. Register schemas via inventory. Add round-trip serde tests and accept schema snapshots.
  - Verify: `cargo test -p assay-types -- orchestrate` passes; `cargo test -p assay-core --features orchestrate` compiles; `cargo insta test --review` accepts snapshots
  - Done when: All types compile, serialize round-trip, schema snapshots locked, executor module has public type stubs

- [x] **T02: Split pipeline into setup and execute phases** `est:30m`
  - Why: The executor needs to serialize worktree creation (D018) while parallelizing agent execution. This requires splitting `run_session()` into two phases: setup (spec load + session start + worktree create) under the worktree mutex, and execute (harness + agent + gate + merge) in parallel.
  - Files: `crates/assay-core/src/pipeline.rs`
  - Do: Extract `SetupResult` struct (session_id, spec_entry, worktree_info, work_session). Extract `setup_session()` (stages 1-2) and `execute_session()` (stages 3-6) as public functions. Rewrite `run_session()` as the composition of both. All existing pipeline tests must continue to pass unchanged.
  - Verify: `cargo test -p assay-core -- pipeline` — all existing tests pass; `run_session()` behavior is identical
  - Done when: `setup_session()` and `execute_session()` are public; `run_session()` delegates to them; zero test changes needed

- [x] **T03: Implement executor dispatch loop with state persistence** `est:60m`
  - Why: This is the core of S02 — the bounded-concurrency DAG-driven dispatch loop that runs sessions in parallel, propagates failures, and persists state
  - Files: `crates/assay-core/src/orchestrate/executor.rs`, `crates/assay-core/src/orchestrate/mod.rs`
  - Do: Implement `run_orchestrated()` taking manifest, `OrchestratorConfig`, `&HarnessWriter`, and session runner closure `Fn(...)`. Build `DependencyGraph`, generate ULID run_id, create `.assay/orchestrator/<run_id>/` dir. Enter `std::thread::scope` with condvar dispatch loop: lock shared state → call `ready_set()` → spawn up to `max_concurrency` threads → each thread calls `setup_session` under worktree mutex then `execute_session` in parallel → record outcome → call `mark_skipped_dependents` on failure → persist state snapshot → notify condvar. Wrap thread bodies in `catch_unwind`. Support `FailurePolicy::Abort` (cancel remaining on first failure). Return `OrchestratorResult`.
  - Verify: `cargo test -p assay-core --features orchestrate -- orchestrate::executor` with at least 3 basic tests (all-parallel, linear chain, single failure skip)
  - Done when: `run_orchestrated()` compiles, basic tests pass proving dispatch loop, failure propagation, and state persistence work

- [x] **T04: Comprehensive executor tests and full verification** `est:45m`
  - Why: Proves the executor contract thoroughly — parallel execution, dependency ordering, bounded concurrency, abort policy, state file readability — and confirms the full build passes
  - Files: `crates/assay-core/src/orchestrate/executor.rs`
  - Do: Add tests covering: (1) diamond DAG dispatch ordering, (2) bounded concurrency enforcement via AtomicUsize peak tracking, (3) FailurePolicy::Abort stops all dispatch, (4) panic in session runner caught and treated as failure, (5) state.json is valid OrchestratorStatus with correct per-session states, (6) single-session manifest works through orchestrator, (7) all sessions independent runs fully parallel. Run `just ready` and fix any lint/fmt issues.
  - Verify: `cargo test -p assay-core --features orchestrate -- orchestrate::executor` — all tests pass; `just ready` — full suite green
  - Done when: ≥12 executor tests covering all must-haves; `just ready` passes clean; `cargo test -p assay-core` (without feature) still passes

## Files Likely Touched

- `crates/assay-types/src/orchestrate.rs` (new)
- `crates/assay-types/src/lib.rs`
- `crates/assay-core/src/orchestrate/executor.rs` (new)
- `crates/assay-core/src/orchestrate/mod.rs`
- `crates/assay-core/src/pipeline.rs`
- `crates/assay-types/tests/snapshots/` (new schema snapshots)
