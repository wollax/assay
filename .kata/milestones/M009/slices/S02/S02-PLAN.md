# S02: Pipeline span instrumentation

**Goal:** Add `#[instrument]` spans to the 5 public pipeline functions and `tracing::info_span!` wrapping each of the 6 pipeline stage blocks, so a single-agent pipeline run produces a span tree with named stages, spec slug fields, and timing.
**Demo:** A test using `tracing-test` captures subscriber output from a mock pipeline call and asserts that span names `pipeline::setup_session`, `pipeline::execute_session`, `spec_load`, `worktree_create`, `harness_config`, `agent_launch`, `gate_evaluate`, and `merge_check` appear with correct fields.

## Must-Haves

- `tracing-test` added as workspace dev-dependency and to assay-core dev-dependencies
- `#[instrument]` on all 5 public pipeline functions (`setup_session`, `execute_session`, `run_session`, `run_manifest`, `launch_agent`) with appropriate `skip` and `fields`
- `tracing::info_span!` wrapping each of the 6 stage blocks inside `setup_session` and `execute_session` with stage-specific names and fields
- `tracing::info!` events at successful stage completions and `tracing::warn!` on error paths
- Integration tests using `#[traced_test]` asserting span names appear in captured output
- Existing 13 pipeline unit tests + 4 integration tests remain green
- `just ready` passes

## Proof Level

- This slice proves: contract (span names, fields, and nesting verified via test subscriber capture)
- Real runtime required: no (mock/test subscriber captures spans)
- Human/UAT required: no

## Verification

- `cargo test -p assay-core pipeline_spans` — new integration test file asserting span names in captured output
- `cargo test -p assay-core --lib -- pipeline` — existing 13 unit tests still pass
- `cargo test -p assay-core --test pipeline_streaming` — existing 4 integration tests still pass
- `just ready` — full workspace green

## Observability / Diagnostics

- Runtime signals: `#[instrument]` spans emit `new_span` / `close_span` events through the active subscriber; `tracing::info!("stage completed")` events at each stage boundary with elapsed time
- Inspection surfaces: `RUST_LOG=assay_core::pipeline=debug` shows all pipeline span activity; span fields (`spec`, `session_id`, `session_count`, `working_dir`) provide structured context
- Failure visibility: `tracing::warn!` events on error paths carry the stage name and error message before `PipelineError` is returned — a future agent can filter `RUST_LOG=assay_core::pipeline=warn` to see only failures
- Redaction constraints: none — pipeline spans contain spec slugs and paths, no secrets

## Integration Closure

- Upstream surfaces consumed: `assay_core::telemetry::init_tracing()` (S01) — subscriber must be active for spans to be recorded; `tracing = "0.1"` macros already available in assay-core
- New wiring introduced in this slice: `#[instrument]` proc-macro annotations on pipeline functions; `info_span!().in_scope()` wrapping stage blocks; `tracing-test` dev-dependency for test assertions
- What remains before the milestone is truly usable end-to-end: S03 (orchestration spans nesting pipeline spans), S04 (JSON export layer for `.assay/traces/`), S05 (OTLP export + TRACEPARENT propagation)

## Tasks

- [x] **T01: Add tracing-test dev-dependency and write span assertion integration tests** `est:20m`
  - Why: Test-first — define the contract (expected span names and fields) before instrumenting. Tests will fail initially (no spans emitted yet), proving the assertions are real.
  - Files: `Cargo.toml` (workspace), `crates/assay-core/Cargo.toml`, `crates/assay-core/tests/pipeline_spans.rs`
  - Do: Add `tracing-test = "0.2"` as workspace dev-dep and to assay-core dev-deps. Write integration test file with `#[traced_test]` tests that call `setup_session` and `run_session` with mock data (spec-not-found paths are fine — the function-level span is entered before error). Assert `logs_contain("pipeline::setup_session")` etc. Tests should fail (red) because no `#[instrument]` exists yet.
  - Verify: `cargo test -p assay-core --test pipeline_spans` compiles but tests fail (expected span names not found)
  - Done when: test file exists, compiles, and fails with "span not found" (not compilation error)

- [x] **T02: Instrument pipeline functions with #[instrument] and stage-level spans** `est:30m`
  - Why: Core implementation — adds the 5 function-level spans and 6 stage-level spans that make the T01 tests pass.
  - Files: `crates/assay-core/src/pipeline.rs`
  - Do: Add `#[instrument]` to 5 public functions per the research instrumentation plan (skip large params, add fields manually). Wrap each stage block in `tracing::info_span!("stage_name").in_scope(|| { ... })`. Add `tracing::info!("stage completed")` after each successful stage and `tracing::warn!` on error paths before returning PipelineError.
  - Verify: `cargo test -p assay-core --test pipeline_spans` — T01 tests now pass; `cargo test -p assay-core --lib -- pipeline` — existing 13 tests still pass; `just ready` green
  - Done when: all T01 span assertion tests pass, all existing pipeline tests pass, `just ready` green

## Files Likely Touched

- `Cargo.toml` — workspace dev-dependency addition
- `crates/assay-core/Cargo.toml` — dev-dependency addition
- `crates/assay-core/src/pipeline.rs` — `#[instrument]` and `info_span!` additions
- `crates/assay-core/tests/pipeline_spans.rs` — new integration test file
