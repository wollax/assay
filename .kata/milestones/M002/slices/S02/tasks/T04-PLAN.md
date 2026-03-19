---
estimated_steps: 4
estimated_files: 2
---

# T04: Comprehensive executor tests and full verification

**Slice:** S02 — Parallel Session Executor
**Milestone:** M002

## Description

Add thorough test coverage for the executor to prove all must-haves: diamond DAG ordering, bounded concurrency enforcement, abort failure policy, panic recovery, state file readability, and single-session compatibility. Run `just ready` to confirm the full build passes cleanly.

## Steps

1. Add concurrency verification tests: (a) 5 independent sessions with `max_concurrency: 2` — use `AtomicUsize` to track peak concurrent executions, assert peak ≤ 2. (b) 8 independent sessions with default concurrency — assert all complete. Mock session runners use `thread::sleep(Duration::from_millis(50))` to create overlap windows.
2. Add DAG ordering tests: (a) diamond graph (A → {B,C} → D) — verify D starts only after both B and C complete (use timestamps or ordering log via `Arc<Mutex<Vec<String>>>`). (b) Mixed deps and independent: verify independent sessions run even when dependent chains are blocked.
3. Add failure and edge-case tests: (a) `FailurePolicy::Abort` — first failure stops all new dispatch, already-running sessions finish, remaining are marked as skipped. (b) Panic in mock session runner — caught and treated as `SessionOutcome::Failed`, dependents skipped. (c) Single-session manifest through orchestrator — works correctly. (d) All sessions fail — every dependent skipped, no deadlock.
4. Add state persistence test: read `.assay/orchestrator/<run_id>/state.json` after orchestrator completes, deserialize as `OrchestratorStatus`, assert correct phase and per-session states. Run `just ready` and fix any fmt/lint/deny issues.

## Must-Haves

- [ ] Bounded concurrency test proves peak ≤ configured limit
- [ ] Diamond DAG test proves D executes only after B and C
- [ ] FailurePolicy::Abort test proves remaining sessions skipped
- [ ] Panic recovery test proves panics become Failed outcomes
- [ ] State persistence test proves state.json is valid OrchestratorStatus
- [ ] Single-session test proves backward compatibility
- [ ] `just ready` passes clean (fmt, lint, test, deny)
- [ ] `cargo test -p assay-core` (without orchestrate feature) still passes

## Verification

- `cargo test -p assay-core --features orchestrate -- orchestrate::executor` — ≥12 tests pass
- `cargo test -p assay-core` (without feature) — existing tests unaffected
- `just ready` — full suite green

## Observability Impact

- Signals added/changed: None — tests validate existing observability surfaces
- How a future agent inspects this: Tests serve as executable documentation of executor behavior
- Failure state exposed: Tests verify that failure state is correctly exposed in SessionOutcome and OrchestratorStatus

## Inputs

- T03 output: `run_orchestrated()` with basic tests passing
- T01 output: `OrchestratorStatus` type for state.json deserialization
- Mock session runner pattern established in T03's basic tests

## Expected Output

- `crates/assay-core/src/orchestrate/executor.rs` — ≥12 total tests (3 from T03 + 9+ new) proving all executor contracts
- `just ready` passes clean — the slice is complete
