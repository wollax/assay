---
estimated_steps: 5
estimated_files: 1
---

# T02: Instrument pipeline functions with #[instrument] and stage-level spans

**Slice:** S02 — Pipeline span instrumentation
**Milestone:** M009

## Description

Add `#[instrument]` to the 5 public pipeline functions and wrap each of the 6 stage blocks inside `setup_session` and `execute_session` with `tracing::info_span!().in_scope()`. Add `tracing::info!` events at stage completion and `tracing::warn!` on error paths. This makes the T01 integration tests pass and delivers the full pipeline span instrumentation.

## Steps

1. Add `#[instrument]` to `setup_session`:
   - `#[instrument(name = "pipeline::setup_session", skip(config), fields(spec = %manifest_session.spec))]`
   - Wrap Stage 1 (SpecLoad) body in `tracing::info_span!("spec_load", spec = %manifest_session.spec).in_scope(|| { ... })`
   - Wrap Stage 2 (WorktreeCreate) body in `tracing::info_span!("worktree_create", spec = %manifest_session.spec).in_scope(|| { ... })`
   - Add `tracing::info!("stage completed")` after each successful stage
   - Add `tracing::warn!` before returning PipelineError on error paths

2. Add `#[instrument]` to `execute_session`:
   - `#[instrument(name = "pipeline::execute_session", skip(config, harness_writer, setup), fields(spec = %manifest_session.spec, session_id = %setup.session_id))]`
   - Wrap Stage 3 (HarnessConfig) in `tracing::info_span!("harness_config", spec = %manifest_session.spec).in_scope(|| { ... })`
   - Wrap Stage 4 (AgentLaunch) in `tracing::info_span!("agent_launch", spec = %manifest_session.spec).in_scope(|| { ... })`
   - Wrap Stage 5 (GateEvaluate) in `tracing::info_span!("gate_evaluate", spec = %spec_name).in_scope(|| { ... })`
   - Wrap Stage 6 (MergeCheck) in `tracing::info_span!("merge_check", spec = %spec_name).in_scope(|| { ... })`
   - Add `tracing::info!` / `tracing::warn!` at stage boundaries

3. Add `#[instrument]` to `run_session`:
   - `#[instrument(name = "pipeline::run_session", skip(config, harness_writer), fields(spec = %manifest_session.spec))]`

4. Add `#[instrument]` to `run_manifest`:
   - `#[instrument(name = "pipeline::run_manifest", skip(config, harness_writer, manifest), fields(session_count = manifest.sessions.len()))]`

5. Add `#[instrument]` to `launch_agent`:
   - `#[instrument(name = "pipeline::launch_agent", skip(cli_args, timeout), fields(working_dir = %working_dir.display()))]`
   - Note: `launch_agent_streaming` is NOT instrumented (TUI concern, different thread model — S03 scope if needed)

## Must-Haves

- [ ] `#[instrument]` on `setup_session`, `execute_session`, `run_session`, `run_manifest`, `launch_agent` with correct `name`, `skip`, and `fields`
- [ ] `tracing::info_span!` wrapping all 6 stage blocks: `spec_load`, `worktree_create`, `harness_config`, `agent_launch`, `gate_evaluate`, `merge_check`
- [ ] `tracing::info!("stage completed")` after each successful stage
- [ ] `tracing::warn!` on error paths before returning PipelineError
- [ ] T01 integration tests (`cargo test -p assay-core --test pipeline_spans`) all pass (green)
- [ ] Existing 13 pipeline unit tests pass
- [ ] Existing 4 pipeline_streaming integration tests pass
- [ ] `just ready` green

## Verification

- `cargo test -p assay-core --test pipeline_spans` — all span assertion tests pass
- `cargo test -p assay-core --lib -- pipeline` — all 13 existing tests pass
- `cargo test -p assay-core --test pipeline_streaming` — all 4 existing tests pass
- `cargo fmt --all -- --check` — clean
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `just ready` — green

## Observability Impact

- Signals added/changed: 5 function-level spans + 6 stage-level spans + info/warn events at stage boundaries
- How a future agent inspects this: `RUST_LOG=assay_core::pipeline=debug` shows full span tree with timing; `RUST_LOG=assay_core::pipeline=warn` shows only failures
- Failure state exposed: `tracing::warn!` events on every error path carry stage name + error message before PipelineError propagation

## Inputs

- `crates/assay-core/src/pipeline.rs` — current pipeline code (read in planning)
- `crates/assay-core/tests/pipeline_spans.rs` — T01 test file defining the span contract
- S02-RESEARCH.md — instrumentation plan table with exact span names, fields, and skip params

## Expected Output

- `crates/assay-core/src/pipeline.rs` — `#[instrument]` on 5 functions, `info_span!` on 6 stages, info/warn events at boundaries
