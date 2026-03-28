---
id: T01
parent: S03
milestone: M013
provides:
  - metrics feature enabled on opentelemetry, opentelemetry_sdk, opentelemetry-otlp workspace deps
  - 4 contract tests defining the metrics API surface (red state until T02)
  - Verified SdkMeterProvider, Counter, Histogram types resolve under telemetry feature
  - Verified default build unaffected (zero new OTel deps)
key_files:
  - Cargo.toml (workspace root — metrics feature added to 3 OTel deps)
  - crates/assay-core/tests/otel_metrics.rs (4 contract tests)
key_decisions: []
patterns_established:
  - "Contract test pattern: tests reference not-yet-implemented API (record_session_launched, init_metric_handles, etc.) — compile failure is expected red state"
observability_surfaces:
  - none (test-only task)
duration: 8min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T01: Contract tests and workspace dep setup

**Added `metrics` feature to OTel workspace deps and wrote 4 red-state contract tests defining the metrics API surface**

## What Happened

Added the `metrics` feature to the three OTel workspace dependencies (`opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp`) in root `Cargo.toml`. Verified the `assay-core` `telemetry` feature gate propagates the metrics capability transitively — no crate-level Cargo.toml changes needed.

Created `otel_metrics.rs` with 4 contract tests:
1. `test_meter_provider_init_compiles` — constructs `SdkMeterProvider` with periodic exporter, creates counter, calls `.add(1)`
2. `test_histogram_recording` — creates histogram, calls `.record(42.0)`
3. `test_recording_functions_noop_without_init` — calls `record_session_launched()` etc. without init, expects no panic
4. `test_init_metrics_populates_handles` — calls `init_metric_handles(&meter)` then exercises all recording functions

Tests 3 and 4 reference the public API surface T02 will implement (`record_session_launched`, `record_gate_evaluated`, `record_merge_attempted`, `record_gate_eval_latency_ms`, `record_agent_run_duration_ms`, `init_metric_handles`). They currently fail to compile with 11 "cannot find function" errors — expected red state.

## Verification

- `cargo build -p assay-core --features telemetry` — compiles successfully
- `cargo build -p assay-core` — compiles with zero new OTel deps
- `cargo tree -p assay-core --features telemetry -e features | grep metrics` — confirms `metrics` feature on `opentelemetry_sdk`, `opentelemetry-otlp`, `opentelemetry-proto`
- `cargo test --test otel_metrics --features telemetry --no-run` — fails with 11 expected "cannot find function" errors for T02 API surface
- Test file has exactly 4 test functions with `#[cfg(feature = "telemetry")]` module gate

## Diagnostics

None — test-only task. Read `otel_metrics.rs` for the API contract T02 must satisfy.

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `Cargo.toml` — Added `metrics` feature to `opentelemetry`, `opentelemetry_sdk`, `opentelemetry-otlp` workspace deps
- `crates/assay-core/tests/otel_metrics.rs` — 4 contract tests defining metrics API surface
