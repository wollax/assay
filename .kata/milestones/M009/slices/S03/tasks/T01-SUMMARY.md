---
id: T01
parent: S03
milestone: M009
provides:
  - 5 red-state integration tests defining the orchestration span contract
  - Mock helpers: mock_manifest(), mock_pipeline_config(), instant_runner()
  - Span assertion patterns using `{` suffix to distinguish named spans from module paths
key_files:
  - crates/assay-core/tests/orchestrate_spans.rs
key_decisions:
  - "Used `orchestrate::dag{` (with opening brace) instead of `orchestrate::dag` in assertions to prevent false positives from module-path matching in tracing-test output"
patterns_established:
  - "orchestrate_spans.rs mock helpers pattern: mock_manifest(n), mock_pipeline_config() with tempdir, instant_runner closure — same style as pipeline_spans.rs but without requiring real git repos"
observability_surfaces:
  - "`cargo test -p assay-core --test orchestrate_spans --features orchestrate` is the canonical orchestration span contract check"
duration: 8min
verification_result: passed
completed_at: 2026-03-25T02:32:00Z
blocker_discovered: false
---

# T01: Create red-state orchestration span integration tests

**5 red-state integration tests defining the orchestration span contract for DAG, Mesh, Gossip executors and merge runner using tracing-test with mock session runners**

## What Happened

Created `crates/assay-core/tests/orchestrate_spans.rs` with the `#![cfg(feature = "orchestrate")]` gate. Added three helper functions: `mock_manifest(n)` builds a `RunManifest` with N independent sessions, `mock_pipeline_config()` returns a `PipelineConfig` backed by a tempdir with `.assay` created, and `instant_runner` is a closure returning `Ok(PipelineResult)` with minimal fields.

Wrote 5 tests asserting span names: `test_dag_root_span_emitted`, `test_dag_session_span_emitted`, `test_mesh_root_span_emitted`, `test_gossip_root_span_emitted`, and `test_merge_root_span_emitted`. Each test calls the real orchestration function with the mock runner and asserts `logs_contain("orchestrate::<mode>{")` (or `merge::run{`).

Used `{` suffix in all assertion strings to match only named spans with fields, not module-path text like `assay_core::orchestrate::mesh:` which caused a false positive with bare `"orchestrate::mesh"`.

## Verification

- `cargo test -p assay-core --test orchestrate_spans --features orchestrate` — compiles, runs, all 5 tests FAIL on `logs_contain` assertions (span names not yet emitted by production code)
- `cargo test -p assay-core --lib` — 691 passed, 0 failed (no regressions)

## Diagnostics

- `cargo test -p assay-core --test orchestrate_spans --features orchestrate` is the canonical check — once production instrumentation is added in T02+, these 5 tests should flip to green

## Deviations

- Task plan specified bare `logs_contain("orchestrate::dag")` etc. Changed to `logs_contain("orchestrate::dag{")` (with `{` suffix) because bare substring matched module paths in tracing-test output, causing `test_mesh_root_span_emitted` to pass as a false positive. The `{` suffix ensures only actual named spans (which include field braces) are matched.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/tests/orchestrate_spans.rs` — New integration test file with 5 red-state span assertion tests and mock helpers
