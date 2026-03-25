---
estimated_steps: 4
estimated_files: 3
---

# T01: Add tracing-test dev-dependency and write span assertion integration tests

**Slice:** S02 — Pipeline span instrumentation
**Milestone:** M009

## Description

Test-first: add the `tracing-test` crate as a dev-dependency and write integration tests that assert expected span names appear in subscriber output when pipeline functions are called. The tests will initially fail (red state) because no `#[instrument]` annotations exist yet — this proves the assertions are real and not vacuously passing.

The tests exercise code paths that enter the function-level span before hitting an early error (e.g. spec-not-found), which is sufficient to prove the `#[instrument]` span is created. Stage-level spans inside `setup_session` and `execute_session` are tested via paths that reach those stages.

## Steps

1. Add `tracing-test = "0.2"` to workspace `[workspace.dev-dependencies]` in root `Cargo.toml`
2. Add `tracing-test.workspace = true` to `[dev-dependencies]` in `crates/assay-core/Cargo.toml`
3. Create `crates/assay-core/tests/pipeline_spans.rs` with `#[traced_test]` integration tests:
   - `test_setup_session_emits_span`: call `setup_session()` with a non-existent spec (triggers SpecLoad error but span is entered first). Assert `logs_contain("pipeline::setup_session")`.
   - `test_run_session_emits_span`: call `run_session()` with a non-existent spec. Assert `logs_contain("pipeline::run_session")`.
   - `test_run_manifest_emits_span`: call `run_manifest()` with one session. Assert `logs_contain("pipeline::run_manifest")`.
   - `test_setup_session_emits_spec_load_span`: same setup_session call, assert `logs_contain("spec_load")`.
4. Verify: `cargo test -p assay-core --test pipeline_spans` compiles but tests fail with assertion errors (span names not found in output)

## Must-Haves

- [ ] `tracing-test = "0.2"` in workspace dev-dependencies
- [ ] `tracing-test.workspace = true` in assay-core dev-dependencies
- [ ] `crates/assay-core/tests/pipeline_spans.rs` exists with 4+ `#[traced_test]` tests
- [ ] Tests compile successfully
- [ ] Tests fail (red) because expected span names are not yet emitted

## Verification

- `cargo test -p assay-core --test pipeline_spans` — compiles and runs, tests fail (expected)
- `cargo test -p assay-core --lib` — existing tests unaffected (still pass)

## Observability Impact

- Signals added/changed: None (test infrastructure only)
- How a future agent inspects this: run `cargo test -p assay-core --test pipeline_spans` to check span contract compliance
- Failure state exposed: test assertions name the exact missing span, making it clear which `#[instrument]` annotation is absent

## Inputs

- `crates/assay-core/src/pipeline.rs` — function signatures for `setup_session`, `run_session`, `run_manifest` (to understand parameter types for test construction)
- S02-RESEARCH.md — instrumentation plan and `tracing-test` usage patterns

## Expected Output

- `Cargo.toml` — `tracing-test` added to workspace dev-deps
- `crates/assay-core/Cargo.toml` — `tracing-test` added to dev-deps
- `crates/assay-core/tests/pipeline_spans.rs` — new test file (red state)
