---
estimated_steps: 4
estimated_files: 1
---

# T02: Split pipeline into setup and execute phases

**Slice:** S02 — Parallel Session Executor
**Milestone:** M002

## Description

Refactor `run_session()` into two public functions — `setup_session()` (spec load + work session start + worktree create) and `execute_session()` (harness config + agent launch + gate evaluate + merge check). This enables the executor to serialize worktree creation via a mutex (D018) while running agent execution in parallel. `run_session()` becomes a thin composition of both, preserving all existing behavior and tests.

## Steps

1. Define `SetupResult` struct in `pipeline.rs` containing: `session_id: String`, `spec_name: String`, `spec_entry: SpecEntry`, `worktree_info: WorktreeInfo`, `stage_timings: Vec<StageTiming>`. This is the handoff from setup to execute. Note: `SpecEntry` must be `Send` for `std::thread::scope` — verify this. `WorktreeInfo` is already `Clone + Send`.
2. Extract `setup_session()` as a public function: takes `&ManifestSession` + `&PipelineConfig`, runs stages 1-2 (SpecLoad + WorktreeCreate), returns `Result<SetupResult, PipelineError>`. On failure after session start, abandons the session (same as current behavior).
3. Extract `execute_session()` as a public function: takes `&ManifestSession`, `&PipelineConfig`, `&HarnessWriter`, `SetupResult`, runs stages 3-6 (HarnessConfig + AgentLaunch + GateEvaluate + MergeCheck), returns `Result<PipelineResult, PipelineError>`. On failure, abandons the session via session_id from SetupResult.
4. Rewrite `run_session()` to call `setup_session()` then `execute_session()`, forwarding results. Verify all existing pipeline tests pass without modification.

## Must-Haves

- [ ] `setup_session()` is public and runs stages 1-2 only
- [ ] `execute_session()` is public and runs stages 3-6 only
- [ ] `SetupResult` carries all state needed by execute_session (no re-loading)
- [ ] `run_session()` behavior is identical to before (composition of setup + execute)
- [ ] All existing pipeline tests pass without any changes
- [ ] `SetupResult` types are `Send` (required for thread::scope)

## Verification

- `cargo test -p assay-core -- pipeline` — all 10 existing pipeline tests pass unchanged
- `cargo test -p assay-core --features orchestrate` — compiles with feature
- No new test failures anywhere in `cargo test -p assay-core`

## Observability Impact

- Signals added/changed: None — pure refactor of existing function
- How a future agent inspects this: Same as before — PipelineError with stage context
- Failure state exposed: No change — same error paths, same session abandonment

## Inputs

- `crates/assay-core/src/pipeline.rs` — current `run_session()` with stages 1-6
- `crates/assay-types/src/worktree.rs` — `WorktreeInfo` type (verify Send)
- `crates/assay-core/src/spec.rs` — `SpecEntry` type (verify Send)

## Expected Output

- `crates/assay-core/src/pipeline.rs` — `setup_session()`, `execute_session()`, `SetupResult` added; `run_session()` preserved as composition; all existing tests pass
