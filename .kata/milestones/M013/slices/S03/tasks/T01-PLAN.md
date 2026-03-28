---
estimated_steps: 5
estimated_files: 3
---

# T01: Contract tests and workspace dep setup

**Slice:** S03 — OTel Metrics
**Milestone:** M013

## Description

Add the `metrics` feature to the OTel workspace deps and gate them through `assay-core`'s `telemetry` feature. Write red-state contract tests that define the API contract for metrics initialization, counter/histogram recording, and feature-flag isolation. These tests will fail until T02 provides the implementation.

## Steps

1. In root `Cargo.toml`, add `metrics` to the feature lists of `opentelemetry`, `opentelemetry_sdk`, and `opentelemetry-otlp` workspace deps (alongside existing features like `rt-tokio`, `http-proto`, `hyper-client`).
2. In `crates/assay-core/Cargo.toml`, verify the `telemetry` feature already gates `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`. The metrics feature propagates transitively from workspace deps — no crate-level change needed beyond confirming the existing `telemetry` feature list is sufficient.
3. Verify `cargo build -p assay-core --features telemetry` compiles successfully with the new metrics types available (e.g. `opentelemetry_sdk::metrics::SdkMeterProvider` resolves).
4. Verify `cargo build -p assay-core` (default features) still compiles with zero new OTel deps.
5. Create `crates/assay-core/tests/otel_metrics.rs` with these contract tests (all `#[cfg(feature = "telemetry")]`):
   - `test_meter_provider_init_compiles`: Construct `SdkMeterProvider` via builder with periodic exporter, create a counter, call `.add(1)`, verify no panic. Confirms the `metrics` feature is correctly enabled.
   - `test_histogram_recording`: Create a histogram via meter, call `.record(42.0)`, verify no panic. Confirms histogram API is available.
   - `test_recording_functions_noop_without_init`: Call `record_session_launched()` etc. without metrics initialization — must not panic. Confirms the OnceLock guard works.
   - `test_init_metrics_populates_handles`: Call `init_metric_handles` with a real `Meter`, then call each recording function — verify counters increment (no panics, OnceLock populated).

## Must-Haves

- [ ] `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp` workspace deps have `metrics` feature
- [ ] `cargo build -p assay-core --features telemetry` compiles
- [ ] `cargo build -p assay-core` (no features) compiles with zero new deps
- [ ] `crates/assay-core/tests/otel_metrics.rs` exists with 4 contract tests
- [ ] Tests reference the public API surface that T02 will implement (`record_session_launched`, `init_metric_handles`, etc.)

## Verification

- `cargo build -p assay-core --features telemetry` — compiles
- `cargo build -p assay-core` — compiles, no new deps
- `cargo tree -p assay-core --features telemetry -i opentelemetry_sdk` — shows `metrics` feature
- Test file exists and has the 4 test functions (they may fail to compile until T02 — that's expected red state)

## Observability Impact

- Signals added/changed: None (test-only task)
- How a future agent inspects this: Read test file for the API contract
- Failure state exposed: Compilation errors indicate feature gate misconfiguration

## Inputs

- `Cargo.toml` (workspace root) — current OTel dep features
- `crates/assay-core/Cargo.toml` — current `telemetry` feature gate
- S03-RESEARCH.md — API patterns for `SdkMeterProvider`, `Counter`, `Histogram`

## Expected Output

- `Cargo.toml` — OTel deps with `metrics` feature added
- `crates/assay-core/tests/otel_metrics.rs` — 4 contract tests defining the metrics API surface
