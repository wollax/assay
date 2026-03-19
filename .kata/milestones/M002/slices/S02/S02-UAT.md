# S02: Parallel Session Executor — UAT

**Milestone:** M002
**Written:** 2026-03-17

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: The executor is an internal engine consumed by S06 (CLI + MCP). All contract points are proven by 18 automated tests covering DAG ordering, bounded concurrency, failure propagation, panic recovery, and state persistence. No user-facing surface exists yet — that arrives in S06.

## Preconditions

- Rust toolchain installed (stable)
- Repository cloned with `orchestrate` feature available
- S01 (DAG validation) merged

## Smoke Test

Run `cargo test -p assay-core --features orchestrate -- orchestrate::executor::tests::three_independent_sessions_all_complete` — should pass in <2s, proving the executor dispatches parallel sessions and persists valid state.

## Test Cases

### 1. Parallel dispatch of independent sessions

1. Run `cargo test -p assay-core --features orchestrate -- three_independent_sessions_all_complete`
2. **Expected:** 3 sessions complete with `SessionOutcome::Completed`, state.json shows `Completed` phase

### 2. Dependency ordering respected

1. Run `cargo test -p assay-core --features orchestrate -- linear_chain_executes_in_order`
2. **Expected:** A completes before B starts, B completes before C starts

### 3. Diamond DAG ordering

1. Run `cargo test -p assay-core --features orchestrate -- diamond_dag_d_runs_after_b_and_c`
2. **Expected:** D starts only after both B and C have ended

### 4. Failure propagation skips dependents

1. Run `cargo test -p assay-core --features orchestrate -- failure_skips_dependents_independent_completes`
2. **Expected:** A fails → B skipped with reason referencing A, C (independent) completes

### 5. Bounded concurrency enforced

1. Run `cargo test -p assay-core --features orchestrate -- bounded_concurrency_enforced`
2. **Expected:** Peak concurrent sessions ≤ configured max_concurrency (2), actually reaches 2

### 6. Abort policy stops dispatch

1. Run `cargo test -p assay-core --features orchestrate -- abort_policy_stops_dispatch`
2. **Expected:** Only the failing session runs, state.json shows `Aborted` phase

### 7. Panic recovery

1. Run `cargo test -p assay-core --features orchestrate -- panic_in_runner_caught_as_failure`
2. **Expected:** Panic caught as `Failed` with "panic:" prefix, dependent skipped

### 8. State persistence correctness

1. Run `cargo test -p assay-core --features orchestrate -- state_persistence_has_correct_fields`
2. **Expected:** state.json deserializes as `OrchestratorStatus` with correct run_id, phase, failure_policy, per-session states, timing fields

## Edge Cases

### Single session through orchestrator

1. Run `cargo test -p assay-core --features orchestrate -- single_session_through_orchestrator`
2. **Expected:** Solo session works correctly, backward compatible with single-session manifests

### All sessions fail without deadlock

1. Run `cargo test -p assay-core --features orchestrate -- all_sessions_fail_no_deadlock`
2. **Expected:** All fail, dependents skipped, no deadlock, test completes in bounded time

## Failure Signals

- Any test in `orchestrate::executor` failing
- `just ready` failing after S02 changes
- State.json not deserializable as `OrchestratorStatus`
- Deadlock in dispatch loop (test hangs indefinitely)
- Peak concurrency exceeding configured bound

## Requirements Proved By This UAT

- R020 (Multi-agent orchestration) — DAG-driven parallel executor with dependency ordering, failure propagation, and bounded concurrency proven by 18 automated tests. Partial proof: S06 capstone will prove end-to-end CLI routing.

## Not Proven By This UAT

- Real agent subprocess execution (mock runners only)
- Real worktree creation under mutex (mock runners bypass two-phase split)
- CLI routing of multi-session manifests to orchestrator (S06)
- MCP tool integration for orchestrate_status (S06)
- Sequential merge after parallel execution (S03)
- End-to-end multi-session manifest through real CLI (S06)

## Notes for Tester

All tests use mock session runners that simulate success/failure/timing without spawning real agents. This is intentional — the executor's contract is dispatch ordering, concurrency bounds, failure propagation, and state persistence, not agent lifecycle. Real agent integration is proven in S06.
