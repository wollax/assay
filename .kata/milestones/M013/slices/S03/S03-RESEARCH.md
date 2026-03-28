# S03: OTel Metrics — Research

**Date:** 2026-03-28

## Summary

OTel metrics for S03 adds counters and histograms alongside the existing tracing spans in `assay-core::telemetry`. The workspace already has `opentelemetry 0.31.0`, `opentelemetry_sdk 0.31.0`, and `opentelemetry-otlp 0.31.1` pinned as workspace deps, but only `rt-tokio` feature is enabled on `opentelemetry_sdk` — the `metrics` feature is NOT enabled. Similarly, `opentelemetry-otlp` only has `http-proto` and `hyper-client` features — the `metrics` feature is missing.

The core work is: (1) add `metrics` feature to `opentelemetry`, `opentelemetry_sdk`, and `opentelemetry-otlp` workspace deps behind the `telemetry` feature gate on `assay-core`; (2) build an `init_metrics()` function that creates a `SdkMeterProvider` with a `PeriodicReader` backed by the OTLP `MetricExporter`; (3) store the `SdkMeterProvider` in `TracingGuard` for shutdown; (4) create global metric handles via `OnceLock<Counter<u64>>` / `OnceLock<Histogram<f64>>`; (5) instrument the five metric sites in pipeline.rs and orchestration code.

The API surface is well-proven in the 0.31 SDK. `SdkMeterProvider::builder().with_periodic_exporter(exporter).build()` is the primary construction pattern. `MetricExporter::builder().with_http().with_endpoint(endpoint).build()` mirrors the existing `SpanExporter` pattern exactly. The `Meter` from the provider creates `Counter<u64>` and `Histogram<f64>` instruments. This is low-risk mechanical work following established patterns in the codebase.

## Recommendation

Mirror the existing `build_otel_layer` pattern exactly:
1. Add a `build_otel_metrics` function behind `#[cfg(feature = "telemetry")]` that creates `MetricExporter` → `SdkMeterProvider` (via `with_periodic_exporter`).
2. Store the `SdkMeterProvider` in `TracingGuard` as `Option<SdkMeterProvider>` — shutdown on drop (D179).
3. Use module-level `OnceLock<Counter<u64>>` and `OnceLock<Histogram<f64>>` for the five global metric handles. Initialize them in a new `init_metrics(meter: &Meter)` helper called after the provider is built.
4. Create thin recording functions (`record_session_launched()`, `record_gate_evaluated()`, etc.) that check the OnceLock and no-op when metrics are not initialized. These are callable from non-telemetry code without `cfg` guards.
5. Place the five recording calls at: `run_session` entry (sessions_launched), `gate_evaluate` span entry/exit (gates_evaluated + gate_eval_latency_ms), `merge_completed_sessions` entry (merges_attempted), `launch_agent` entry/exit (agent_run_duration_ms).

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| OTLP metric export | `opentelemetry_otlp::MetricExporter` with `http-proto` feature | Same transport as span export (D144); avoids reqwest conflict |
| Periodic metric reader | `opentelemetry_sdk::metrics::PeriodicReader` | Standard SDK component; `with_periodic_exporter` shorthand on `MeterProviderBuilder` handles construction |
| Global metric handles | `std::sync::OnceLock` | Zero-dep, standard library; initialized once, read from any thread |
| Shutdown coordination | `SdkMeterProvider::shutdown()` in TracingGuard::drop | Mirrors D147 pattern for `SdkTracerProvider`; flush-before-drop is guaranteed |

## Existing Code and Patterns

- `crates/assay-core/src/telemetry.rs` — The entire init/guard/layer pattern lives here. `init_tracing()` builds the subscriber, `build_otel_layer()` constructs OTLP span exporter. Metrics code slots in parallel to spans code.
- `crates/assay-core/src/telemetry.rs:TracingGuard` — Already has `#[cfg(feature = "telemetry")] _tracer_provider: Option<SdkTracerProvider>`. Add `_meter_provider: Option<SdkMeterProvider>` with the same pattern. Drop impl shuts down meters THEN traces (D179).
- `crates/assay-core/src/pipeline.rs` — Has `#[instrument]` on all pipeline functions. Counter increments go at the entry of `run_session`, inside the `gate_evaluate` span, inside `launch_agent`. Histogram records go at the end of `gate_evaluate` and `launch_agent` using `Instant::now()` elapsed timing.
- `crates/assay-core/src/orchestrate/merge_runner.rs:merge_completed_sessions` — Entry point for merge phase; `merges_attempted` counter goes here.
- `crates/assay-core/Cargo.toml` — The `telemetry` feature already gates all four OTel crates. Need to add `metrics` feature to `opentelemetry`, `opentelemetry_sdk`, and `opentelemetry-otlp` deps.

## Constraints

- **Feature gate:** All metrics code MUST be behind `#[cfg(feature = "telemetry")]`. Default build must have zero new OTel deps added. The existing pattern uses `cfg` at function and field level — follow it exactly.
- **D144 transport:** Must use `http-proto` + `hyper-client` (no reqwest). `MetricExporter::builder().with_http()` is the correct path — same as `SpanExporter::builder().with_http()`.
- **D179:** `MeterProvider` stored in `TracingGuard`; drop triggers `meter_provider.shutdown()` before `tracer_provider.shutdown()`. Metrics must flush first because some metric pipelines reference traces.
- **D007 sync core:** No async in business logic. `SdkMeterProvider` with `PeriodicReader` handles async export internally (same as `BatchSpanProcessor`).
- **Workspace deps:** `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp` are workspace deps in root `Cargo.toml`. Feature additions go there, not in crate-local Cargo.toml.
- **`just ready` green:** Must pass all 1516+ existing tests. No regression.

## Common Pitfalls

- **Missing `metrics` feature on `opentelemetry-otlp`** — Without it, `MetricExporter` is not compiled. The `metrics` feature on `opentelemetry-otlp` transitively enables `opentelemetry/metrics` and `opentelemetry_sdk/metrics`. Must be added to the workspace dep AND gated through the `telemetry` feature in `assay-core/Cargo.toml`.
- **PeriodicReader requires tokio runtime** — `PeriodicReader` spawns a background task. The existing `rt-tokio` feature on `opentelemetry_sdk` provides the async runtime support. This is already enabled, so no additional work needed. But if the runtime is not available at init time (unlikely), `build()` will panic. Same risk profile as the existing span exporter.
- **Shutdown ordering in TracingGuard::drop** — Metrics provider must shut down before tracer provider. If reversed, the periodic reader's internal span for flushing may fail because the tracer provider is gone. The existing `Drop` impl for `TracingGuard` handles tracer only; extend it to call `meter_provider.shutdown()` FIRST, then `tracer_provider.shutdown()`.
- **OnceLock ergonomics in recording functions** — `OnceLock::get()` returns `Option<&T>`. The recording functions must check for `None` and silently no-op. This allows non-telemetry builds and builds where OTLP init failed to call the same recording functions without conditional compilation at every call site.
- **Histogram unit convention** — OTel convention: latency histograms should use milliseconds with `f64`. Use `Instant::elapsed().as_secs_f64() * 1000.0` for conversion. Counter instruments should use `u64`.

## Open Risks

- **opentelemetry-otlp `metrics` + `http-proto` + `hyper-client` combination untested in this workspace** — The span export path has been validated but metrics export through the same transport has not. Risk: a compile error or runtime panic from an unexpected feature gate interaction. Mitigation: verify compilation early in T01.
- **PeriodicReader default interval (60s) may miss short-lived CLI runs** — If `assay gate run` completes in 2 seconds, the periodic reader may not have flushed metrics before `TracingGuard` drops. Mitigation: `SdkMeterProvider::shutdown()` forces a final flush. This is the same mechanism that works for `SdkTracerProvider`.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| OpenTelemetry Rust | — | none found (niche Rust crate, no agent skill exists) |
| Ratatui | — | not relevant to this slice |

## Sources

- `opentelemetry-otlp 0.31.1` source code at `~/.cargo/registry/` — confirmed `MetricExporter::builder().with_http()` API, `metrics` feature gate, and `http-proto` transport compatibility
- `opentelemetry_sdk 0.31.0` source code — confirmed `SdkMeterProvider::builder().with_periodic_exporter(exporter).build()` construction pattern and `shutdown()` method
- `opentelemetry 0.31.0` source code — confirmed `Meter::u64_counter()`, `Meter::f64_histogram()`, `Counter::add()`, `Histogram::record()` API surface
- Existing `crates/assay-core/src/telemetry.rs` — `build_otel_layer()` pattern for OTLP span export; `TracingGuard` for shutdown; `cfg(feature = "telemetry")` gating
- D126, D143, D144, D147, D179 from DECISIONS.md — architectural constraints for metrics integration
