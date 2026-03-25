# S02: Pipeline span instrumentation — Research

**Date:** 2026-03-24
**Domain:** Rust tracing / OpenTelemetry span instrumentation
**Confidence:** HIGH

## Summary

S02 adds `#[instrument]` spans to the five public pipeline functions (`setup_session`, `execute_session`, `run_session`, `run_manifest`, `launch_agent`) and injects `tracing::info_span!` / `tracing::debug!` events at each of the 6 pipeline stage boundaries inside `setup_session` and `execute_session`. The crate already depends on `tracing = "0.1"` (with the `attributes` feature implicitly included) and `tracing-subscriber = "0.3"` — no new workspace dependencies are needed for instrumentation itself.

The primary verification challenge is asserting that spans exist with correct names/fields in tests. The project currently has zero `#[instrument]` usage. Two approaches exist: (1) `tracing-test` crate (dev-dependency, provides `#[traced_test]` macro + `logs_contain()` assertion), or (2) a custom `tracing_subscriber::Layer` that captures span metadata into a shared `Vec<String>`. The `tracing-test` crate (0.2.6, ~2M downloads) is the simplest approach and avoids hand-rolling capture infrastructure. Adding it as a dev-dependency to assay-core is the recommended path.

The pipeline module (`crates/assay-core/src/pipeline.rs`, 1180 lines, 13 existing tests) is fully synchronous (D007). `#[instrument]` works naturally with sync code — no async span bridging needed. The existing `StageTiming` records already capture per-stage durations; spans add structured names, fields, and parent-child nesting on top of that.

## Recommendation

1. Add `tracing-test = "0.2"` as a workspace dev-dependency and to `assay-core`'s `[dev-dependencies]`
2. Add `#[instrument]` to the 5 public pipeline functions with `skip` on large/non-Debug params
3. Add `tracing::info_span!("stage_name").in_scope(|| { ... })` wrapping each stage block inside `setup_session` and `execute_session`
4. Write integration tests using `#[traced_test]` that call mock pipelines and assert span names appear in captured output
5. Do NOT touch orchestration code (S03 scope) or export code (S04/S05 scope)

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Capturing spans in tests | `tracing-test` crate (0.2.6) | Provides `#[traced_test]` + `logs_contain!()` — avoids custom subscriber layer for test assertions |
| Per-stage timing | Existing `StageTiming` + `stage_timings: Vec<StageTiming>` on `PipelineResult` / `SetupResult` | Spans complement timing records but don't replace them — keep both |
| Subscriber initialization | `assay_core::telemetry::init_tracing()` from S01 | Production subscriber is already layered; `#[instrument]` emits spans to whatever subscriber is active |

## Existing Code and Patterns

- `crates/assay-core/src/pipeline.rs` — 1180 lines, 6 stages, 5 public functions. Each stage has a `let stage_start = Instant::now()` block followed by error handling and `stage_timings.push(StageTiming { ... })`. Spans wrap these existing blocks.
- `crates/assay-core/src/telemetry.rs` — S01's centralized subscriber. Uses `registry().with(filter).with(fmt_layer)`. `#[instrument]` emits spans through this subscriber automatically.
- `crates/assay-core/src/pipeline.rs:408` — `build_harness_profile()` is a pure function. Instrumenting it is low value (no I/O, fast). Skip it.
- `crates/assay-core/src/pipeline.rs:334` — `launch_agent_streaming()` spawns a thread. `#[instrument]` on the outer function captures the span before the thread spawn; the inner thread work is NOT automatically spanned (thread boundary). This is acceptable — the span captures "streaming agent launched" with timing.
- `crates/assay-core/tests/pipeline_streaming.rs` — Existing integration tests spawn real `sh` subprocesses. New span tests should NOT depend on real subprocesses — use the existing unit test pattern with mock data or `#[traced_test]` on simpler paths.

## Constraints

- **D007 (sync core):** All pipeline functions are synchronous. `#[instrument]` works natively — no `async-trait` or `.instrument()` futures needed.
- **D001 (zero-trait):** `HarnessWriter` is `dyn Fn(...)` — cannot add `#[instrument]` to it. Span the call site instead.
- **`skip` required on large params:** `PipelineConfig` and `ManifestSession` are not `Debug`-friendly for span recording (contain paths, nested structs). Use `#[instrument(skip(config, harness_writer, setup))]` and add relevant fields manually via `fields(spec = %manifest_session.spec)`.
- **`tracing = "0.1"` already in workspace deps:** The `#[instrument]` proc-macro is included in `tracing`'s default features. No Cargo.toml changes needed for the macro itself.
- **Existing test count:** 13 unit tests in `pipeline.rs` + 4 integration tests in `pipeline_streaming.rs`. New span tests must not break existing tests.

## Common Pitfalls

- **`#[instrument]` on functions with `dyn Fn` params** — The `dyn Fn` param (`harness_writer: &HarnessWriter`) does not implement `Debug`. Must use `skip(harness_writer)` or the compilation will fail with a missing `Debug` bound error.
- **Span fields from borrowed data** — `manifest_session.spec` is a `String` field borrowed from a reference. Use `fields(spec = %manifest_session.spec)` (Display) not `fields(spec = manifest_session.spec)` (which tries Debug on the parent struct).
- **Thread-crossing spans** — `launch_agent_streaming` spawns a `std::thread`. The inner thread does NOT automatically inherit the parent span. This is expected for S02 — cross-thread parenting is S03's scope (orchestration). For S02, the span on the outer function captures the "launch" event and timing.
- **`#[traced_test]` initializes its own subscriber** — It installs a test subscriber that captures output. If `init_tracing()` was already called (e.g., in a test setup), `try_init` silently skips. `#[traced_test]` sets up its own subscriber before the test function body, so it wins the race in unit tests. Integration tests that call `init_tracing()` explicitly should NOT use `#[traced_test]` — use a dedicated subscriber capture pattern instead.
- **Span names default to function name** — `#[instrument]` uses the function name as the span name (e.g., `setup_session`). To override for clarity, use `#[instrument(name = "pipeline::setup_session")]`. The boundary map specifies span names like `spec_load`, `worktree_create` etc. — these are per-stage spans inside the function, not the function-level span. Use `tracing::info_span!("spec_load")` for stage-level spans.

## Open Risks

- **`tracing-test` compatibility with `tracing 0.1.44`** — The crate's latest release (0.2.6) targets `tracing 0.1` and `tracing-subscriber 0.3`. These match the workspace versions exactly. Low risk.
- **Span output format in tests** — `tracing-test` captures formatted output (not structured spans). Assertions use string matching (`logs_contain("setup_session")`), which is fragile if the fmt layer changes format. Acceptable for contract tests — if a more robust approach is needed, `tracing-capture` (0.2.0-beta.1) provides structured span capture, but it's beta. Stick with `tracing-test` for now.
- **Performance overhead** — Span creation is ~50ns per span. With 6 stages per pipeline run, this adds ~300ns — negligible vs. the seconds-to-minutes agent execution time. No risk.

## Instrumentation Plan

### Function-level spans (via `#[instrument]`)

| Function | Span name | Fields | Skip |
|----------|-----------|--------|------|
| `setup_session` | `pipeline::setup_session` | `spec = %manifest_session.spec` | `config` |
| `execute_session` | `pipeline::execute_session` | `spec = %manifest_session.spec, session_id = %setup.session_id` | `config, harness_writer, setup` — add fields manually |
| `run_session` | `pipeline::run_session` | `spec = %manifest_session.spec` | `config, harness_writer` |
| `run_manifest` | `pipeline::run_manifest` | `session_count = manifest.sessions.len()` | `config, harness_writer, manifest` — add field manually |
| `launch_agent` | `pipeline::launch_agent` | `working_dir = %working_dir.display()` | `cli_args, timeout` |

### Stage-level spans (via `tracing::info_span!().in_scope()`)

| Stage | Span name | Fields |
|-------|-----------|--------|
| Stage 1 | `spec_load` | `spec = %manifest_session.spec` |
| Stage 2 | `worktree_create` | `spec = %manifest_session.spec` |
| Stage 3 | `harness_config` | `spec = %manifest_session.spec` |
| Stage 4 | `agent_launch` | `spec = %manifest_session.spec` |
| Stage 5 | `gate_evaluate` | `spec = %spec_name` |
| Stage 6 | `merge_check` | `spec = %spec_name, base_branch` |

### Events at stage boundaries

- `tracing::info!("stage completed")` after each successful stage (with timing from `StageTiming`)
- `tracing::warn!("stage failed")` on error paths (before returning `PipelineError`)

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Distributed tracing | `wshobson/agents@distributed-tracing` (3.3K installs) | available — general distributed tracing skill, not Rust-specific. Low relevance for this Rust `tracing` crate work. |
| Rust tracing | none found | no Rust-specific tracing skill exists |

## Sources

- `crates/assay-core/src/pipeline.rs` — read in full (1180 lines): 5 public functions, 6 stages, 13 tests
- `crates/assay-core/src/telemetry.rs` — S01's subscriber (165 lines): `init_tracing(TracingConfig) -> TracingGuard`
- `crates/assay-core/Cargo.toml` — tracing deps: `tracing`, `tracing-appender`, `tracing-subscriber` all at workspace versions
- `Cargo.toml` workspace — `tracing = "0.1"`, `tracing-subscriber = "0.3"` with `fmt` + `env-filter` features
- `cargo search tracing-test` — version 0.2.6, compatible with tracing 0.1 / tracing-subscriber 0.3
- S01 summary — forward intelligence on subscriber architecture and testing patterns
