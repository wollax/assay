---
estimated_steps: 5
estimated_files: 2
---

# T02: Instrument DAG executor and merge runner with tracing spans

**Slice:** S03 — Orchestration span instrumentation
**Milestone:** M009

## Description

Add tracing spans to `run_orchestrated()` in `executor.rs` and `merge_completed_sessions()` in `merge_runner.rs`. The core challenge is cross-thread span parenting: `Span::current()` must be captured before `thread::scope` and cloned into each worker closure, then re-entered via `parent_span.in_scope(|| { ... })`. This makes pipeline spans (from S02) automatically nest under orchestration per-session spans.

## Steps

1. In `executor.rs`, add `use tracing::{info_span, Span};`. Wrap the body of `run_orchestrated()` (after parameter binding) in `info_span!("orchestrate::dag", session_count = session_count, mode = "dag").in_scope(|| { ... })`. Capture `let parent_span = Span::current();` inside that scope, before `thread::scope`.
2. Inside `scope.spawn(move || { ... })` in executor.rs, clone `parent_span` into the closure capture list, then wrap the worker body in `parent_span.in_scope(|| { let _session_span = info_span!("orchestrate::dag::session", session_name = %session_name).entered(); ... })`. This nests the session span under the DAG root span across the thread boundary.
3. In `merge_runner.rs`, add `use tracing::{info_span, info, warn};`. Wrap the body of `merge_completed_sessions()` in `info_span!("merge::run", session_count = ordered.len(), strategy = %config.strategy).in_scope(|| { ... })`. Inside the per-session merge loop, wrap each iteration in `info_span!("merge::session", session_name = %session.name).in_scope(|| { ... })`.
4. Add `info_span!("merge::conflict_resolution", session_name = %session_name)` around the conflict handler invocation block in the merge loop.
5. Run `cargo test -p assay-core --test orchestrate_spans` to verify DAG and merge tests pass. Run `cargo test -p assay-core --lib` and `cargo test -p assay-core --test orchestrate_integration` for regression check.

## Must-Haves

- [ ] `run_orchestrated()` emits `orchestrate::dag` root span with `session_count` and `mode` fields
- [ ] Per-session worker spans `orchestrate::dag::session` with `session_name` field, parented to the root span via `Span::current()` capture + `.in_scope()`
- [ ] `merge_completed_sessions()` emits `merge::run` root span with `session_count` and `strategy` fields
- [ ] Per-session merge span `merge::session` with `session_name` field
- [ ] Conflict resolution span `merge::conflict_resolution` around handler calls
- [ ] DAG and merge span tests from T01 now pass
- [ ] All existing `orchestrate_integration` tests pass (zero regressions)

## Verification

- `cargo test -p assay-core --test orchestrate_spans` — DAG and merge tests pass
- `cargo test -p assay-core --lib` — all existing unit tests pass
- `cargo test -p assay-core --test orchestrate_integration` — no regressions

## Observability Impact

- Signals added/changed: `orchestrate::dag` root span, `orchestrate::dag::session` per-session spans, `merge::run` root span, `merge::session` per-session spans, `merge::conflict_resolution` spans
- How a future agent inspects this: `RUST_LOG=assay_core::orchestrate=debug` shows full DAG span tree; `cargo test -p assay-core --test orchestrate_spans` checks span contract
- Failure state exposed: Session failures now appear within their named session span, making it clear which session failed in multi-session DAG runs

## Inputs

- `crates/assay-core/tests/orchestrate_spans.rs` — T01 red-state tests (now to be made green)
- `crates/assay-core/src/pipeline.rs` — S02 pattern reference for `#[instrument]` and `info_span!` usage
- S02 summary — `info_span!(name, fields).in_scope(|| { ... })` is the established pattern

## Expected Output

- `crates/assay-core/src/orchestrate/executor.rs` — instrumented with root span + per-session cross-thread spans
- `crates/assay-core/src/orchestrate/merge_runner.rs` — instrumented with root span + per-session + conflict resolution spans
