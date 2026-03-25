---
id: T02
parent: S03
milestone: M009
provides:
  - orchestrate::dag root span with session_count and mode fields on run_orchestrated()
  - orchestrate::dag::session per-session worker spans with cross-thread parenting via Span::current() capture
  - merge::run root span with session_count and strategy fields on merge_completed_sessions()
  - merge::session per-session spans inside the merge loop
  - merge::conflict_resolution spans around both resolution-enabled and default conflict handler paths
key_files:
  - crates/assay-core/src/orchestrate/executor.rs
  - crates/assay-core/src/orchestrate/merge_runner.rs
key_decisions:
  - "Used ?config.strategy (Debug) instead of %config.strategy (Display) in merge::run span because MergeStrategy does not implement Display"
  - "Used _parent_guard = parent_span.enter() + nested _session_span for cross-thread span parenting rather than Span::in_scope closure, to keep the existing code structure intact"
  - "Added info!() events inside each span because tracing-test logs_contain() checks formatted event output, not bare span creation"
patterns_established:
  - "Cross-thread span parenting: capture Span::current() before thread::scope, clone into each closure, re-enter with .enter(), then create child spans — established in executor.rs"
observability_surfaces:
  - "RUST_LOG=assay_core::orchestrate=debug shows full DAG span tree with session names"
  - "cargo test -p assay-core --test orchestrate_spans --features orchestrate checks span contract (3 of 5 tests now pass — DAG + merge)"
duration: 12min
verification_result: passed
completed_at: 2026-03-24T00:00:00Z
blocker_discovered: false
---

# T02: Instrument DAG executor and merge runner with tracing spans

**Added orchestrate::dag root + per-session cross-thread spans to executor.rs and merge::run + per-session + conflict_resolution spans to merge_runner.rs**

## What Happened

Instrumented `run_orchestrated()` in `executor.rs` with an `orchestrate::dag` root span (carrying `session_count` and `mode` fields) and per-session `orchestrate::dag::session` worker spans. The cross-thread span parenting challenge was solved by capturing `Span::current()` before `std::thread::scope`, cloning into each worker closure, re-entering with `.enter()`, then creating the child session span inside that scope.

Instrumented `merge_completed_sessions()` in `merge_runner.rs` with a `merge::run` root span (carrying `session_count` and `strategy` fields), per-session `merge::session` spans inside the merge loop, and `merge::conflict_resolution` spans around both the resolution-enabled and default conflict handler invocation paths.

Added `info!()` events inside each span so that tracing-test's `logs_contain()` can detect span presence via the formatted event context prefix.

## Verification

- `cargo test -p assay-core --test orchestrate_spans --features orchestrate` — 3 passed (test_dag_root_span_emitted, test_dag_session_span_emitted, test_merge_root_span_emitted), 2 failed (mesh + gossip — expected, those are T03/T04 scope)
- `cargo test -p assay-core --lib --features orchestrate` — 780 passed, 0 failed (zero regressions)
- `cargo test -p assay-core --test orchestrate_integration --features orchestrate` — 5 passed, 0 failed (zero regressions)

## Diagnostics

- `RUST_LOG=assay_core::orchestrate=debug` shows the full DAG span tree with session names
- `cargo test -p assay-core --test orchestrate_spans --features orchestrate` is the canonical span contract check
- Session failures now appear within their named session span, making it clear which session failed in multi-session DAG runs

## Deviations

- Used `?` (Debug format) for `strategy` field in `merge::run` span because `MergeStrategy` does not implement `Display` — the plan specified `%config.strategy` which requires Display
- Added `info!()` events inside spans — not in the original plan but required for tracing-test `logs_contain()` to detect span presence

## Known Issues

None

## Files Created/Modified

- `crates/assay-core/src/orchestrate/executor.rs` — Added tracing imports, orchestrate::dag root span, cross-thread parent span capture, per-session orchestrate::dag::session spans with info events
- `crates/assay-core/src/orchestrate/merge_runner.rs` — Added tracing imports, merge::run root span, per-session merge::session spans, merge::conflict_resolution spans around handler calls, info events
