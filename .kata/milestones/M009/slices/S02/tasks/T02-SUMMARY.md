---
id: T02
parent: S02
milestone: M009
provides:
  - "#[instrument] on 5 public pipeline functions (setup_session, execute_session, run_session, run_manifest, launch_agent)"
  - "info_span! wrapping 6 stage blocks (spec_load, worktree_create, harness_config, agent_launch, gate_evaluate, merge_check)"
  - "tracing::info!(\"stage completed\") after each successful stage"
  - "tracing::warn! on error paths before returning PipelineError"
  - "tracing-test no-env-filter feature enabled for cross-crate span assertion"
key_files:
  - crates/assay-core/src/pipeline.rs
  - Cargo.toml
key_decisions:
  - "Enabled tracing-test no-env-filter feature: required for cross-crate span assertions since tests live in pipeline_spans crate but spans are emitted from assay_core"
patterns_established:
  - "info_span!(name, fields).in_scope(|| { ... }) pattern for wrapping stage blocks with structured span context"
  - "warn!(stage = name, error = %e, message) pattern for error-path tracing before PipelineError return"
observability_surfaces:
  - "RUST_LOG=assay_core::pipeline=debug shows full span tree with timing for all 5 functions and 6 stages"
  - "RUST_LOG=assay_core::pipeline=warn shows only failure events with stage name and error message"
duration: 15min
verification_result: passed
completed_at: 2026-03-24T00:00:00Z
blocker_discovered: false
---

# T02: Instrument pipeline functions with #[instrument] and stage-level spans

**Added #[instrument] spans to 5 pipeline functions and info_span! wrappers around all 6 stage blocks with info/warn events at boundaries**

## What Happened

Instrumented `crates/assay-core/src/pipeline.rs` with tracing:

1. Added `use tracing::{info, info_span, instrument, warn}` import.
2. Added `#[instrument(name = "pipeline::setup_session", skip(config), fields(spec = ...))]` to `setup_session`.
3. Wrapped Stage 1 (SpecLoad) and Stage 2 (WorktreeCreate) bodies in `info_span!("spec_load", ...).in_scope(|| { ... })` and `info_span!("worktree_create", ...).in_scope(|| { ... })`.
4. Added `#[instrument(name = "pipeline::execute_session", skip(config, harness_writer, setup), fields(spec, session_id))]` to `execute_session`.
5. Wrapped Stages 3-6 (harness_config, agent_launch, gate_evaluate, merge_check) in `info_span!` blocks.
6. Added `#[instrument]` to `run_session`, `run_manifest`, and `launch_agent` with appropriate skip/fields.
7. Added `tracing::info!("stage completed")` after each successful stage.
8. Added `tracing::warn!(stage = ..., error = ..., ...)` on every error path before returning PipelineError.
9. Enabled `no-env-filter` feature on `tracing-test` workspace dependency so cross-crate span assertions work.

## Verification

- `cargo test -p assay-core --test pipeline_spans` — 4/4 passed (spec_load span, setup_session span, run_session span, run_manifest span)
- `cargo test -p assay-core --lib -- pipeline` — 20/20 passed (all existing pipeline unit tests)
- `cargo test -p assay-core --test pipeline_streaming` — 4/4 passed
- `cargo fmt --all -- --check` — clean
- `cargo clippy -p assay-core --lib -- -D warnings` — clean (pre-existing needless_update warnings in test code are unrelated)

## Diagnostics

- `RUST_LOG=assay_core::pipeline=debug` shows full span tree with timing for all pipeline functions and stages
- `RUST_LOG=assay_core::pipeline=warn` shows only failure events with stage name and error context
- Span fields carry structured context: `spec`, `session_id`, `session_count`, `working_dir`

## Deviations

- Enabled `tracing-test = { version = "0.2", features = ["no-env-filter"] }` in workspace Cargo.toml — this was not in the task plan but is required for the T01 tests to work, since tests run in the `pipeline_spans` crate while spans are emitted from `assay_core`. Without `no-env-filter`, the tracing-test subscriber filters to only the test crate's module path.

## Known Issues

- Pre-existing clippy `needless_update` warnings (11 instances) in test code across the workspace — not introduced by this task, not related to pipeline instrumentation.

## Files Created/Modified

- `crates/assay-core/src/pipeline.rs` — Added #[instrument] on 5 functions, info_span! on 6 stages, info/warn events at boundaries
- `Cargo.toml` — Enabled no-env-filter feature on tracing-test workspace dependency
