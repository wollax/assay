# S02: Pipeline span instrumentation — UAT

**Milestone:** M009
**Written:** 2026-03-24

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: Pipeline spans are verified by test subscriber capture in integration tests — no live runtime or human judgment needed. The span contract is fully mechanical.

## Preconditions

- Rust toolchain installed (`cargo` available)
- Repository checked out with S02 changes

## Smoke Test

Run `cargo test -p assay-core --test pipeline_spans` — all 4 tests should pass, confirming pipeline span names are emitted correctly.

## Test Cases

### 1. Pipeline span names present in subscriber output

1. Run `cargo test -p assay-core --test pipeline_spans`
2. **Expected:** 4/4 tests pass — `pipeline::setup_session`, `pipeline::run_session`, `pipeline::run_manifest`, and `spec_load` span names all captured

### 2. Existing pipeline tests unaffected

1. Run `cargo test -p assay-core --lib -- pipeline`
2. **Expected:** 20/20 existing pipeline unit tests pass with no regressions

### 3. Pipeline streaming tests unaffected

1. Run `cargo test -p assay-core --test pipeline_streaming`
2. **Expected:** 4/4 streaming integration tests pass

### 4. Runtime span visibility

1. Set `RUST_LOG=assay_core::pipeline=debug`
2. Run any pipeline command (e.g. `assay run` with a simple manifest)
3. **Expected:** stderr shows span names (`pipeline::setup_session`, `spec_load`, etc.) with timing and structured fields (`spec`, `session_id`)

## Edge Cases

### Error-path tracing

1. Run a pipeline with a non-existent spec slug
2. Set `RUST_LOG=assay_core::pipeline=warn`
3. **Expected:** stderr shows `warn` events with `stage` and `error` fields before the PipelineError is returned

## Failure Signals

- Any of the 4 `pipeline_spans` tests failing indicates a span name regression
- Missing `tracing::warn!` events on error paths means failures would be invisible to log filtering
- If `RUST_LOG=assay_core::pipeline=debug` shows no output, the subscriber is not initialized (check `init_tracing()`)

## Requirements Proved By This UAT

- R061 (Pipeline span instrumentation) — `#[instrument]` spans on all 5 pipeline functions, `info_span!` on 6 stages, with structured fields and timing, verified by test subscriber capture

## Not Proven By This UAT

- R062 (Orchestration span instrumentation) — S03 scope
- R063 (JSON file trace export) — S04 scope
- R064/R065 (OTLP export / context propagation) — S05 scope
- Full happy-path span tree (would require real git repo + spec + agent) — only error-path span entry is tested

## Notes for Tester

- Test case 4 (runtime span visibility) requires an actual spec and manifest to exercise the full pipeline. If you don't have one, the integration test results from test case 1 are sufficient.
- Pre-existing clippy `needless_update` warnings may appear in workspace builds — they are unrelated to S02.
