---
estimated_steps: 5
estimated_files: 1
---

# T01: Create red-state orchestration span integration tests

**Slice:** S03 — Orchestration span instrumentation
**Milestone:** M009

## Description

Test-first: define the orchestration span contract by writing integration tests that assert expected span names appear in captured `tracing-test` subscriber output. Tests use mock session runners (instant success closures) with minimal manifests. All tests should compile and fail — proving the assertions are real before any instrumentation code is written.

## Steps

1. Create `crates/assay-core/tests/orchestrate_spans.rs` with `#![cfg(feature = "orchestrate")]` gate
2. Add helper functions: `mock_manifest(n_sessions)` returning a `RunManifest` with N sessions (no depends_on), `mock_pipeline_config()` returning a `PipelineConfig` with tempdir paths, and `instant_runner` closure returning `Ok(PipelineResult)` with minimal fields
3. Write `test_dag_root_span_emitted` — calls `run_orchestrated()` with 2-session manifest and instant runner, asserts `logs_contain("orchestrate::dag")`
4. Write `test_dag_session_span_emitted` — same setup, asserts `logs_contain("orchestrate::dag::session")`
5. Write `test_mesh_root_span_emitted` — calls `run_mesh()` with 2-session manifest, asserts `logs_contain("orchestrate::mesh")`
6. Write `test_gossip_root_span_emitted` — calls `run_gossip()` with 2-session manifest, asserts `logs_contain("orchestrate::gossip")`
7. Write `test_merge_root_span_emitted` — calls `merge_completed_sessions()` with empty vec (returns immediately), asserts `logs_contain("merge::run")`

## Must-Haves

- [ ] `orchestrate_spans.rs` compiles with `--features orchestrate`
- [ ] All 5 tests execute and fail (span names not yet emitted by the production code)
- [ ] Mock runner returns `Ok(PipelineResult)` without requiring real git repos or agent processes
- [ ] Tests use `#[traced_test]` + `logs_contain()` pattern from S02

## Verification

- `cargo test -p assay-core --test orchestrate_spans` — compiles, runs, all 5 tests FAIL on `logs_contain` assertions
- `cargo test -p assay-core --lib` — all existing tests still pass (no side effects)

## Observability Impact

- Signals added/changed: None (test-only change)
- How a future agent inspects this: `cargo test -p assay-core --test orchestrate_spans` is the canonical orchestration span contract check
- Failure state exposed: Test failure messages clearly name which span is missing

## Inputs

- `crates/assay-core/tests/pipeline_spans.rs` — reference for `tracing-test` pattern, mock helpers
- S02 summary — `tracing-test` with `no-env-filter` feature (D136) is the required pattern for cross-crate span assertions

## Expected Output

- `crates/assay-core/tests/orchestrate_spans.rs` — new file with 5 red-state integration tests and helper functions
