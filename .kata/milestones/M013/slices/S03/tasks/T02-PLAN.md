---
estimated_steps: 5
estimated_files: 1
---

# T02: build_otel_metrics, init_metric_handles, and TracingGuard integration

**Slice:** S03 — OTel Metrics
**Milestone:** M013

## Description

Implement the core metrics infrastructure in `telemetry.rs`: the meter provider builder, five global metric handles via `OnceLock`, thin recording functions, and `TracingGuard` integration with correct shutdown ordering. This is the implementation that makes T01's contract tests pass.

## Steps

1. In `telemetry.rs`, add `build_otel_metrics(config: &TracingConfig) -> Option<SdkMeterProvider>` behind `#[cfg(feature = "telemetry")]`. Mirror the `build_otel_layer` pattern: read `config.otlp_endpoint`, build `MetricExporter::builder().with_http().with_endpoint(endpoint).build()`, then `SdkMeterProvider::builder().with_periodic_exporter(exporter).build()`. Return `None` when endpoint is `None` or on failure (with `tracing::warn!`).
2. Add five module-level `OnceLock` globals (behind `#[cfg(feature = "telemetry")]`): `SESSIONS_LAUNCHED: OnceLock<Counter<u64>>`, `GATES_EVALUATED: OnceLock<Counter<u64>>`, `MERGES_ATTEMPTED: OnceLock<Counter<u64>>`, `GATE_EVAL_LATENCY: OnceLock<Histogram<f64>>`, `AGENT_RUN_DURATION: OnceLock<Histogram<f64>>`. Add `pub fn init_metric_handles(meter: &Meter)` that creates each instrument via `meter.u64_counter(name).build()` / `meter.f64_histogram(name).build()` and sets the `OnceLock`. Make this `pub` for test access.
3. Add five thin public recording functions (NOT behind `cfg`): `record_session_launched()`, `record_gate_evaluated()`, `record_merges_attempted()`, `record_gate_eval_latency(ms: f64)`, `record_agent_run_duration(ms: f64)`. Each checks the `OnceLock::get()` and calls `.add(1, &[])` or `.record(value, &[])`. When `cfg(not(feature = "telemetry"))`, these are empty functions.
4. Add `_meter_provider: Option<SdkMeterProvider>` to `TracingGuard` behind `#[cfg(feature = "telemetry")]`. Update `Drop` impl: shut down `_meter_provider` FIRST, then `_tracer_provider` (D179). Update `init_tracing`: call `build_otel_metrics`, if `Some`, get a `Meter` from the provider, call `init_metric_handles`, store provider in guard.
5. Verify: `cargo test -p assay-core --test otel_metrics --features telemetry` passes all 4 contract tests. `cargo build -p assay-core` (default) compiles. `cargo test -p assay-core --lib` existing unit tests pass.

## Must-Haves

- [ ] `build_otel_metrics` returns `Some(SdkMeterProvider)` when endpoint configured, `None` otherwise
- [ ] Five `OnceLock` globals populated by `init_metric_handles`
- [ ] Five recording functions are `pub`, callable without `cfg` guards, no-op when uninitialized
- [ ] `TracingGuard` drop: meter shutdown before tracer shutdown (D179)
- [ ] Default (non-telemetry) build compiles — recording functions are empty stubs
- [ ] All 4 contract tests from T01 pass

## Verification

- `cargo test -p assay-core --test otel_metrics --features telemetry` — all 4 tests pass
- `cargo build -p assay-core` — default build clean
- `cargo test -p assay-core --lib` — existing telemetry tests pass
- Grep `_meter_provider` in TracingGuard drop — confirms meter shutdown before tracer

## Observability Impact

- Signals added/changed: `tracing::warn!` on `SdkMeterProvider` init failure; `tracing::debug!` on successful metrics init; `eprintln!` on meter shutdown failure in Drop
- How a future agent inspects this: Check `TracingGuard` drop impl for shutdown ordering; check `OnceLock` globals for metric handle availability
- Failure state exposed: Init failure → `tracing::warn!` with endpoint and error; shutdown failure → `eprintln!` in Drop

## Inputs

- `crates/assay-core/src/telemetry.rs` — existing `build_otel_layer`, `TracingGuard`, `init_tracing` patterns
- `crates/assay-core/tests/otel_metrics.rs` — contract tests from T01 defining the expected API
- S03-RESEARCH.md — `SdkMeterProvider::builder().with_periodic_exporter()`, `Meter::u64_counter()`, `Meter::f64_histogram()` API

## Expected Output

- `crates/assay-core/src/telemetry.rs` — complete metrics infrastructure: `build_otel_metrics`, `init_metric_handles`, 5 recording functions, `TracingGuard` with `_meter_provider`
