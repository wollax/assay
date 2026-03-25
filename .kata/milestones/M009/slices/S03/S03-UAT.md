# S03: Orchestration span instrumentation — UAT

**Milestone:** M009
**Written:** 2026-03-25

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: Span instrumentation is verified by integration tests that capture subscriber output and assert span names. No live runtime or external services needed — mock runners exercise the full orchestration code paths.

## Preconditions

- Rust toolchain installed
- Repository checked out with all dependencies available

## Smoke Test

Run `cargo test -p assay-core --test orchestrate_spans --features orchestrate` — all 5 tests should pass.

## Test Cases

### 1. DAG orchestration spans

1. Run `cargo test -p assay-core --test orchestrate_spans --features orchestrate test_dag_root_span_emitted`
2. Run `cargo test -p assay-core --test orchestrate_spans --features orchestrate test_dag_session_span_emitted`
3. **Expected:** Both tests pass. DAG root span (`orchestrate::dag`) and per-session span (`orchestrate::dag::session`) are emitted.

### 2. Mesh orchestration spans

1. Run `cargo test -p assay-core --test orchestrate_spans --features orchestrate test_mesh_root_span_emitted`
2. **Expected:** Test passes. Mesh root span (`orchestrate::mesh`) is emitted with session_count field.

### 3. Gossip orchestration spans

1. Run `cargo test -p assay-core --test orchestrate_spans --features orchestrate test_gossip_root_span_emitted`
2. **Expected:** Test passes. Gossip root span (`orchestrate::gossip`) is emitted with session_count field.

### 4. Merge runner spans

1. Run `cargo test -p assay-core --test orchestrate_spans --features orchestrate test_merge_root_span_emitted`
2. **Expected:** Test passes. Merge root span (`merge::run`) is emitted.

### 5. Full span tree visibility (manual)

1. Set `RUST_LOG=assay_core::orchestrate=debug`
2. Run any multi-session orchestration (e.g. via `assay run` with a 3-session DAG manifest)
3. **Expected:** Stderr shows nested span context: orchestration root → per-session → pipeline stages. Session names visible in span fields.

## Edge Cases

### Zero-session manifest

1. Create a manifest with no sessions
2. Run orchestration
3. **Expected:** Root span still emitted with `session_count=0`. No per-session spans. No crash.

### Single-session DAG

1. Create a manifest with exactly one session, mode=dag
2. Run orchestration
3. **Expected:** Root span with `session_count=1`, one session span. Pipeline spans nest inside.

## Failure Signals

- `cargo test -p assay-core --test orchestrate_spans --features orchestrate` has any test failure
- `RUST_LOG=assay_core::orchestrate=debug` produces no span context in stderr output during a multi-session run
- Existing orchestration integration tests (`orchestrate_integration`) regress

## Requirements Proved By This UAT

- R062 — Orchestration span instrumentation: all three modes (DAG, Mesh, Gossip) produce root + per-session spans; merge runner produces root + per-session + conflict resolution spans. Proven by 5 integration tests + manual debug output inspection.

## Not Proven By This UAT

- R063 — JSON file trace export (S04 scope)
- R064 — OTLP export (S05 scope)
- R065 — TRACEPARENT propagation to child processes (S05 scope)
- Cross-thread span **parent-child relationship verification** — tests assert span name presence, not hierarchical nesting. Full nesting is verified visually via RUST_LOG output or will be structurally verifiable after S04 (JSON export).

## Notes for Tester

- Test case 5 (full span tree visibility) requires a real multi-session manifest and agent runner. This is best tested after S04/S05 when trace files provide structured inspection. For now, RUST_LOG debug output is the inspection surface.
- The `{` suffix in test assertions (e.g. `orchestrate::dag{`) is intentional — it prevents false positives from module path matching.
