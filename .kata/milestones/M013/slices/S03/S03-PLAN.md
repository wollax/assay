# S03: OTel Metrics

**Goal:** Add OTel counters and histograms alongside existing tracing spans, feature-gated behind `telemetry`. `MeterProvider` stored in `TracingGuard` for clean shutdown. Recording functions callable from non-telemetry code without `cfg` guards.
**Demo:** `cargo build --features telemetry` compiles with metrics; a contract test confirms counter increments and histogram records; `just ready` green.

## Must-Haves

- `metrics` feature added to `opentelemetry`, `opentelemetry_sdk`, and `opentelemetry-otlp` workspace deps, gated through the `telemetry` feature in `assay-core/Cargo.toml`
- `build_otel_metrics(config) -> Option<SdkMeterProvider>` function behind `#[cfg(feature = "telemetry")]`
- `TracingGuard._meter_provider: Option<SdkMeterProvider>` — drop calls `meter_provider.shutdown()` BEFORE `tracer_provider.shutdown()` (D179)
- Five global metric handles via `OnceLock`: `sessions_launched` (Counter<u64>), `gates_evaluated` (Counter<u64>), `merges_attempted` (Counter<u64>), `gate_eval_latency_ms` (Histogram<f64>), `agent_run_duration_ms` (Histogram<f64>)
- Thin recording functions (`record_session_launched()`, etc.) that no-op when metrics not initialized — callable without `cfg` guards
- Five recording call sites: `run_session` entry, `gate_evaluate` span entry+exit, `merge_completed_sessions` entry, `launch_agent` entry+exit
- `cargo build` (default features) has zero new OTel deps
- `just ready` green with all existing tests passing

## Proof Level

- This slice proves: contract (metrics init, counter increment, histogram record, shutdown ordering, feature isolation)
- Real runtime required: no (contract tests with in-process meter provider)
- Human/UAT required: yes — real OTLP collector (Jaeger/Tempo) receiving metrics is UAT only

## Verification

- `cargo test -p assay-core --test otel_metrics --features telemetry` — contract tests for meter init, counter/histogram recording, shutdown
- `cargo build -p assay-core` — default build compiles with zero new OTel deps
- `cargo build -p assay-core --features telemetry` — telemetry build compiles with metrics support
- `cargo tree -p assay-core --features telemetry -i opentelemetry_sdk | grep metrics` — confirms `metrics` feature is enabled
- `just ready` — all 1516+ existing tests pass

## Observability / Diagnostics

- Runtime signals: `tracing::warn!` on `MeterProvider` init failure (graceful degradation); `tracing::debug!` on successful metrics init
- Inspection surfaces: OTel counters/histograms exported to configured OTLP endpoint; no local inspection surface (metrics are push-only to collector)
- Failure visibility: `MeterProvider::shutdown()` error logged via `eprintln!` in `TracingGuard::drop` (mirrors existing tracer pattern); init failure warning includes endpoint and error
- Redaction constraints: none (no secrets in metrics payloads)

## Integration Closure

- Upstream surfaces consumed: `crates/assay-core/src/telemetry.rs` (existing `init_tracing`, `TracingGuard`, `build_otel_layer`); `crates/assay-core/src/pipeline.rs` (instrumentation sites); `crates/assay-core/src/orchestrate/merge_runner.rs` (`merge_completed_sessions`)
- New wiring introduced in this slice: `build_otel_metrics` + `init_metric_handles` called from `init_tracing` when telemetry enabled; recording functions called from pipeline/merge code paths; `SdkMeterProvider` stored in `TracingGuard` for shutdown
- What remains before the milestone is truly usable end-to-end: S04 (wizard runnable criteria) is independent; real OTLP collector verification is UAT

## Tasks

- [x] **T01: Contract tests and workspace dep setup** `est:30m`
  - Why: Establishes the test contract (red state) and adds `metrics` feature to workspace deps — validates compilation before any metrics code is written
  - Files: `Cargo.toml` (workspace root), `crates/assay-core/Cargo.toml`, `crates/assay-core/tests/otel_metrics.rs`
  - Do: Add `metrics` feature to `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp` workspace deps. Gate the new features through `telemetry` in `assay-core/Cargo.toml`. Write red-state contract tests: (1) `SdkMeterProvider` construction + counter increment, (2) histogram recording, (3) default build dep isolation via `cargo tree`. Verify `cargo build --features telemetry` compiles with metrics types available.
  - Verify: `cargo build -p assay-core --features telemetry` compiles; `cargo build -p assay-core` compiles with zero new deps; test file exists (tests will fail until T02 provides the API)
  - Done when: workspace deps have `metrics` feature; `assay-core` `telemetry` feature gates them; contract test file committed; compilation green

- [x] **T02: build_otel_metrics, init_metric_handles, and TracingGuard integration** `est:45m`
  - Why: Core metrics infrastructure — builds the meter provider, creates global metric handles, wires into TracingGuard for shutdown, and connects to init_tracing
  - Files: `crates/assay-core/src/telemetry.rs`
  - Do: Add `build_otel_metrics(config) -> Option<SdkMeterProvider>` behind `#[cfg(feature = "telemetry")]` using `MetricExporter::builder().with_http()` and `SdkMeterProvider::builder().with_periodic_exporter()`. Add `init_metric_handles(meter: &Meter)` that populates five `OnceLock` globals. Add `TracingGuard._meter_provider: Option<SdkMeterProvider>` field. Update `Drop` to shut down meter provider BEFORE tracer provider. Add five thin recording functions (`record_session_launched`, `record_gate_evaluated`, `record_merges_attempted`, `record_gate_eval_latency`, `record_agent_run_duration`) that check OnceLock and no-op when None. Wire `build_otel_metrics` + `init_metric_handles` into `init_tracing`. Ensure default (non-telemetry) build still compiles.
  - Verify: `cargo test -p assay-core --test otel_metrics --features telemetry` — contract tests pass; `cargo build -p assay-core` — default build clean; `cargo test -p assay-core --lib` — existing telemetry unit tests pass
  - Done when: All contract tests from T01 pass; `TracingGuard` shuts down meters then traces; recording functions are pub and callable without cfg guards

- [ ] **T03: Instrument pipeline and merge code paths** `est:30m`
  - Why: Places the five recording calls at their instrumentation sites — the actual metric emission that makes the feature useful
  - Files: `crates/assay-core/src/pipeline.rs`, `crates/assay-core/src/orchestrate/merge_runner.rs`
  - Do: Add `record_session_launched()` at `run_session` entry. Add `record_gate_evaluated()` + `Instant`-based `record_gate_eval_latency(elapsed_ms)` in the `gate_evaluate` span scope. Add `record_merges_attempted()` at `merge_completed_sessions` entry. Add `Instant`-based `record_agent_run_duration(elapsed_ms)` wrapping `launch_agent` call. Use `Instant::elapsed().as_secs_f64() * 1000.0` for histogram values. All calls are unconditional (no cfg guards) — recording functions no-op when metrics not initialized. Run `just ready` to verify zero regression.
  - Verify: `just ready` — all 1516+ tests pass; `cargo build -p assay-core` — no new deps in default build; grep confirms 5 recording call sites in pipeline.rs and merge_runner.rs
  - Done when: Five metric recording sites wired; `just ready` green; no regressions

## Files Likely Touched

- `Cargo.toml` (workspace root — add `metrics` feature to OTel deps)
- `crates/assay-core/Cargo.toml` (gate new features through `telemetry`)
- `crates/assay-core/src/telemetry.rs` (metrics init, handles, recording fns, TracingGuard)
- `crates/assay-core/src/pipeline.rs` (4 recording call sites)
- `crates/assay-core/src/orchestrate/merge_runner.rs` (1 recording call site)
- `crates/assay-core/tests/otel_metrics.rs` (new contract tests)
