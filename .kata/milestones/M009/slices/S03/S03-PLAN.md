# S03: Orchestration span instrumentation

**Goal:** Instrument DAG, Mesh, Gossip executors and merge runner with tracing spans that produce a nested span tree: orchestration root → per-session → pipeline stages. Cross-thread span parenting works correctly inside `std::thread::scope` workers.
**Demo:** A multi-session orchestration test produces span names `orchestrate::dag`, `orchestrate::dag::session`, `orchestrate::mesh`, `orchestrate::mesh::session`, `orchestrate::gossip`, `orchestrate::gossip::session`, `merge::run`, and `merge::session` in captured subscriber output, with per-session spans nested under the orchestration root.

## Must-Haves

- `#[instrument]` or `info_span!` root span on `run_orchestrated()`, `run_mesh()`, `run_gossip()`, `merge_completed_sessions()`
- Per-session `info_span!` inside `scope.spawn` workers for all three executors, parented to the orchestration root via `Span::current()` capture + `.in_scope()`
- Merge runner per-session span wrapping each merge iteration and conflict resolution span
- All existing orchestration tracing events (info/warn/debug in mesh, gossip, conflict_resolver) automatically nest under the new spans
- Integration tests using `tracing-test` + `#[traced_test]` + `logs_contain()` asserting span names for DAG, Mesh, Gossip, and merge runner paths
- Zero regressions: all existing tests pass, `just ready` green

## Proof Level

- This slice proves: integration (span nesting verified by subscriber capture in multi-session tests with mock runners)
- Real runtime required: no (mock session runners, no real agents)
- Human/UAT required: no

## Verification

- `cargo test -p assay-core --test orchestrate_spans` — all span assertion tests pass (DAG root + session, Mesh root + session, Gossip root + session, merge root + session)
- `cargo test -p assay-core --lib` — all existing unit tests pass (including mesh/gossip module tests)
- `cargo test -p assay-core --test orchestrate_integration` — all existing integration tests pass (no regression)
- `cargo fmt --all -- --check` — clean
- `cargo clippy -p assay-core --all-targets -- -D warnings` — clean
- `just ready` — full workspace green

## Observability / Diagnostics

- Runtime signals: `RUST_LOG=assay_core::orchestrate=debug` shows full orchestration span tree with session names; `RUST_LOG=assay_core::orchestrate=warn` shows only failures
- Inspection surfaces: `cargo test -p assay-core --test orchestrate_spans` is the canonical span contract check
- Failure visibility: Each orchestration root span carries `mode` (dag/mesh/gossip) and `session_count`; per-session spans carry `session_name` and `spec`; merge spans carry `session_name` and `strategy`
- Redaction constraints: none — no secrets in orchestration spans

## Integration Closure

- Upstream surfaces consumed: `tracing` macros (S01), `tracing-test` with `no-env-filter` (S02/D136), `info_span!().in_scope()` pattern (S02)
- New wiring introduced in this slice: orchestration root spans wrap pipeline spans from S02 (pipeline per-session spans nest inside orchestration per-session spans automatically via `Span::current()` propagation)
- What remains before the milestone is truly usable end-to-end: S04 (JSON file export), S05 (OTLP export + TRACEPARENT propagation)

## Tasks

- [ ] **T01: Create red-state orchestration span integration tests** `est:25m`
  - Why: Test-first — define the span contract before adding instrumentation. Tests fail initially (proving assertions are real).
  - Files: `crates/assay-core/tests/orchestrate_spans.rs`
  - Do: Create test file with 5 tests: DAG root span, DAG per-session span, Mesh root span, Gossip root span, merge runner root span. Each uses `#[traced_test]` + `logs_contain()`. DAG/Mesh/Gossip tests use 2-session manifests with instant mock runners. Merge test uses `merge_completed_sessions` with empty completed sessions.
  - Verify: `cargo test -p assay-core --test orchestrate_spans` — compiles but all 5 tests fail (span names not yet emitted)
  - Done when: all 5 tests compile, execute, and fail with clear "assertion failed" on `logs_contain`

- [ ] **T02: Instrument DAG executor and merge runner with tracing spans** `est:30m`
  - Why: Core instrumentation — adds root spans and per-session worker spans to executor.rs and merge_runner.rs with cross-thread span parenting
  - Files: `crates/assay-core/src/orchestrate/executor.rs`, `crates/assay-core/src/orchestrate/merge_runner.rs`
  - Do: Add `use tracing::{info_span, Span}` to executor.rs. Add root `info_span!("orchestrate::dag", session_count, mode = "dag")` wrapping `run_orchestrated()` body. Capture `Span::current()` before `thread::scope`, clone into each worker, use `parent_span.in_scope(|| { ... })` inside `scope.spawn`. Add per-session `info_span!("orchestrate::dag::session", session_name)` inside each worker. Add root `info_span!("merge::run", strategy = %config.strategy, session_count)` wrapping `merge_completed_sessions()` body. Add per-session `info_span!("merge::session", session_name)` inside the merge loop. Add `info_span!("merge::conflict_resolution", session_name)` around conflict handler calls.
  - Verify: `cargo test -p assay-core --test orchestrate_spans` — DAG and merge tests pass. `cargo test -p assay-core --lib` — all existing tests pass.
  - Done when: DAG root span, DAG session span, and merge runner span tests pass; zero regressions

- [ ] **T03: Instrument Mesh and Gossip executors with tracing spans** `est:25m`
  - Why: Completes span coverage across all three orchestration modes
  - Files: `crates/assay-core/src/orchestrate/mesh.rs`, `crates/assay-core/src/orchestrate/gossip.rs`
  - Do: Add root `info_span!("orchestrate::mesh", session_count, mode = "mesh")` wrapping `run_mesh()` body. Capture `Span::current()` before `thread::scope`, clone into each worker, use `parent_span.in_scope(|| { ... })` inside `scope.spawn`. Add per-session `info_span!("orchestrate::mesh::session", session_name)` inside workers. Same pattern for `run_gossip()` with `orchestrate::gossip` / `orchestrate::gossip::session`. Routing thread in mesh and coordinator thread in gossip get their own spans (`orchestrate::mesh::routing`, `orchestrate::gossip::coordinator`).
  - Verify: `cargo test -p assay-core --test orchestrate_spans` — all 5 tests pass. `cargo test -p assay-core --lib` — all existing mesh/gossip tests pass. `cargo fmt --all -- --check` and `cargo clippy -p assay-core --all-targets -- -D warnings` clean. `just ready` green.
  - Done when: all 5 orchestrate_spans tests pass, zero regressions, `just ready` green

## Files Likely Touched

- `crates/assay-core/tests/orchestrate_spans.rs` — new integration test file
- `crates/assay-core/src/orchestrate/executor.rs` — root span + per-session worker spans
- `crates/assay-core/src/orchestrate/merge_runner.rs` — root span + per-session merge span + conflict resolution span
- `crates/assay-core/src/orchestrate/mesh.rs` — root span + per-session worker spans + routing thread span
- `crates/assay-core/src/orchestrate/gossip.rs` — root span + per-session worker spans + coordinator thread span
