---
id: T02
parent: S02
milestone: M002
provides:
  - setup_session() public function running pipeline stages 1-2 (SpecLoad + WorktreeCreate)
  - execute_session() public function running pipeline stages 3-6 (HarnessConfig + AgentLaunch + GateEvaluate + MergeCheck)
  - SetupResult struct carrying handoff state between setup and execute phases
  - run_session() preserved as thin composition of setup_session + execute_session
key_files:
  - crates/assay-core/src/pipeline.rs
key_decisions:
  - SetupResult uses Option<String> session_id pattern (not wrapped) for direct field access in executor
patterns_established:
  - Two-phase pipeline split enabling serialized worktree creation with parallel agent execution via mutex
  - Static Send assertion (const _: () = { ... }) to enforce thread-safety at compile time
observability_surfaces:
  - none (pure refactor — same PipelineError paths and session abandonment behavior)
duration: 8m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Split pipeline into setup and execute phases

**Refactored `run_session()` into `setup_session()` + `execute_session()` with `SetupResult` handoff, enabling serialized worktree creation with parallel agent execution.**

## What Happened

Extracted the six-stage `run_session()` function into two public phases:

1. **`setup_session()`** — Takes `&ManifestSession` + `&PipelineConfig`, runs stages 1-2 (SpecLoad + WorktreeCreate), returns `SetupResult` containing session_id, spec_name, spec_entry, worktree_info, and stage_timings. On failure after session start, abandons the session.

2. **`execute_session()`** — Takes `&ManifestSession`, `&PipelineConfig`, `&HarnessWriter`, and `SetupResult`, runs stages 3-6 (HarnessConfig + AgentLaunch + GateEvaluate + MergeCheck), returns `PipelineResult`. On failure, abandons session via session_id from SetupResult.

3. **`run_session()`** — Rewritten as a one-liner: `setup_session()` piped to `execute_session()`. Identical behavior to before.

Added a compile-time static assertion that `SetupResult: Send` (required for `std::thread::scope` in the executor). `SpecEntry` and `WorktreeInfo` are both `Send` — verified by the assertion compiling successfully.

## Verification

- `cargo test -p assay-core -- pipeline` — all 18 pipeline tests pass unchanged (10 direct pipeline + 8 context pipeline tests)
- `cargo test -p assay-core --features orchestrate` — 706 tests pass, 0 failures
- `cargo test -p assay-core` (without feature) — existing tests pass
- `just ready` — full suite green (fmt, lint, test, deny)
- Zero compiler warnings

### Slice-level verification (partial — T02 is intermediate):
- ✅ `cargo test -p assay-types -- orchestrate` — type round-trip and schema tests pass
- ⬜ `cargo test -p assay-core --features orchestrate -- orchestrate::executor` — executor tests not yet written (T03)
- ✅ `cargo test -p assay-core --features orchestrate` — all core tests pass
- ✅ `cargo test -p assay-core` (without feature) — existing tests pass
- ✅ `just ready` — full suite green
- ⬜ state.json readability test (T03/T04)
- ⬜ failed session dependents skipped test (T03/T04)

## Diagnostics

None — pure refactor with no new observability surfaces. Same PipelineError stage context and session abandonment paths as before.

## Deviations

- Removed `#[allow(unused_assignments)]` annotation on `session_id` in `setup_session()` by changing `let mut session_id: Option<String> = None` to `let session_id = Some(ws.id.clone())` declared at the point of use, eliminating the warning cleanly.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/pipeline.rs` — Added `SetupResult` struct, `setup_session()`, `execute_session()` public functions; rewrote `run_session()` as composition; added static Send assertion
