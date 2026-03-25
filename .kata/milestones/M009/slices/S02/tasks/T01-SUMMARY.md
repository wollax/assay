---
id: T01
parent: S02
milestone: M009
provides:
  - tracing-test dev-dependency in workspace
  - 4 red-state integration tests asserting pipeline span names
  - Test helpers for constructing failing pipeline calls
key_files:
  - crates/assay-core/tests/pipeline_spans.rs
  - Cargo.toml
  - crates/assay-core/Cargo.toml
key_decisions: []
patterns_established:
  - "tracing-test #[traced_test] + logs_contain() pattern for span assertion integration tests"
observability_surfaces:
  - "cargo test -p assay-core --test pipeline_spans — checks span contract compliance"
duration: 5min
verification_result: passed
completed_at: 2026-03-24T00:00:00Z
blocker_discovered: false
---

# T01: Add tracing-test dev-dependency and write span assertion integration tests

**Added tracing-test 0.2 and 4 red-state integration tests asserting pipeline::setup_session, pipeline::run_session, pipeline::run_manifest, and spec_load span names**

## What Happened

Added `tracing-test = "0.2"` to workspace dev-dependencies and wired it into assay-core. Created `crates/assay-core/tests/pipeline_spans.rs` with 4 `#[traced_test]` integration tests that call pipeline functions with non-existent specs (triggering SpecLoad errors) and assert that expected span names appear in subscriber output. All 4 tests compile and fail as expected — the span names don't exist yet because no `#[instrument]` annotations have been added.

Test helpers construct a `PipelineConfig` pointing at non-existent paths and a `ManifestSession` referencing a missing spec, plus a no-op `HarnessWriter`. This triggers the SpecLoad error path which is sufficient to prove span entry.

## Verification

- `cargo test -p assay-core --test pipeline_spans` — compiles, 4 tests run, all 4 fail with assertion errors (expected red state)
- `cargo test -p assay-core --lib -- pipeline` — 20 existing pipeline tests pass (no regressions)

### Slice-level verification (partial — T01 of 3):
- ✅ `cargo test -p assay-core --test pipeline_spans` — compiles and runs (red state expected at this stage)
- ✅ `cargo test -p assay-core --lib -- pipeline` — existing unit tests pass
- ⬜ `cargo test -p assay-core --test pipeline_streaming` — not checked (unrelated to this task's changes)
- ⬜ `just ready` — deferred to final task

## Diagnostics

Run `cargo test -p assay-core --test pipeline_spans` to check span contract compliance. Each test assertion names the exact missing span, making it clear which `#[instrument]` annotation is absent.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `Cargo.toml` — Added `tracing-test = "0.2"` to workspace dev-dependencies
- `crates/assay-core/Cargo.toml` — Added `tracing-test.workspace = true` to dev-dependencies
- `crates/assay-core/tests/pipeline_spans.rs` — New integration test file with 4 red-state span assertion tests
