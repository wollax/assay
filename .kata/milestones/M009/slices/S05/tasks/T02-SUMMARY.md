---
id: T02
parent: S05
milestone: M009
provides:
  - TracingConfig.otlp_endpoint field (unconditional, Option<String>)
  - cfg-gated OTel OTLP layer in init_tracing() subscriber chain
  - TraceContextPropagator set before subscriber init
  - Graceful degradation with tracing::warn! on OTel init failure
  - TracingGuard::drop() calls SdkTracerProvider::shutdown() behind cfg(telemetry)
key_files:
  - crates/assay-core/src/telemetry.rs
  - crates/assay-core/tests/telemetry_otlp.rs
key_decisions:
  - "OTel layer uses Option<L> pattern with tracing-subscriber's .with(Option<L>) for zero type-gymnastics — None is a no-op layer"
  - "SdkTracerProvider stored in TracingGuard for shutdown on drop (otel 0.31 has no global shutdown_tracer_provider function)"
  - "build_otel_layer() extracted as a separate cfg-gated function to keep init_tracing() readable"
  - "WithExportConfig trait import required for .with_endpoint() in otel-otlp 0.31"
patterns_established:
  - "cfg-gated OTel init with graceful degradation: build in closure → match Ok/Err → warn on failure"
  - "TracingGuard owns SdkTracerProvider for deterministic shutdown"
observability_surfaces:
  - "tracing::warn!(endpoint, error) on OTLP init failure — always visible at default log level"
  - "SdkTracerProvider::shutdown() on guard drop ensures pending spans are flushed"
  - "RUST_LOG=opentelemetry=debug shows BatchSpanProcessor behavior"
duration: 12min
verification_result: passed
completed_at: 2026-03-26T02:10:00Z
blocker_discovered: false
---

# T02: OTel tracing layer in init_tracing() with feature-flagged TracingGuard shutdown

**Feature-gated OTLP exporter layer added to init_tracing() subscriber chain with graceful degradation and deterministic shutdown via TracingGuard::drop()**

## What Happened

Extended `telemetry.rs` to conditionally add an OpenTelemetry OTLP export layer when the `telemetry` feature is compiled in and `otlp_endpoint` is configured. The implementation:

1. Added `otlp_endpoint: Option<String>` to `TracingConfig` (unconditional — always present regardless of feature flag).
2. Created `build_otel_layer()` behind `#[cfg(feature = "telemetry")]` that: sets `TraceContextPropagator` as the global propagator, builds an HTTP OTLP `SpanExporter` with the configured endpoint, creates an `SdkTracerProvider` with batch export, sets it as the global provider, and returns a `tracing_opentelemetry` layer.
3. Used `Option<Layer>` pattern with tracing-subscriber's `.with(Option<L>)` to avoid type divergence — when OTel is disabled or init fails, `None` acts as a no-op layer.
4. Stored `SdkTracerProvider` in `TracingGuard` and implemented `Drop` to call `provider.shutdown()` for deterministic span flushing. In otel 0.31, there's no global `shutdown_tracer_provider()` — shutdown is on the provider directly.
5. OTel init failures produce a `tracing::warn!` with endpoint and error context, then fall back to fmt-only subscriber.
6. Updated the T01 red-state integration test `test_otel_layer_init_compiles` to use the new `otlp_endpoint` field — it now passes.

## Verification

- `cargo build -p assay-core --features telemetry` — ✓ compiles
- `cargo build -p assay-cli` — ✓ default build compiles (otlp_endpoint is unconditional, OTel code is cfg-gated)
- `cargo build -p assay-cli --features telemetry` — ✓ compiles
- `cargo test -p assay-core telemetry --features telemetry` — ✓ 4 unit tests pass (including default_config with otlp_endpoint: None)
- `cargo test -p assay-core --test telemetry_otlp --features telemetry` — 1 pass, 1 expected fail:
  - ✓ `test_otel_layer_init_compiles` — PASS (T02 contract met)
  - ✗ `test_traceparent_injected_in_subprocess` — expected FAIL (T03 contract, not this task)
- `cargo tree -p assay-cli | grep opentelemetry` — ✓ empty (no OTel deps in default build)
- `cargo tree -p assay-cli --features telemetry | grep opentelemetry` — ✓ shows full OTel crate tree

## Diagnostics

- OTel init failure: `tracing::warn!(endpoint, error, "OTLP exporter init failed; trace export disabled")` — always visible at default log level
- BatchSpanProcessor behavior: `RUST_LOG=opentelemetry=debug`
- Shutdown errors: printed to stderr via `eprintln!` in TracingGuard::drop()

## Deviations

- Task plan referenced `opentelemetry::global::shutdown_tracer_provider()` — this function does not exist in otel 0.31. Instead, `SdkTracerProvider::shutdown()` is called directly on the stored provider instance. Same effect, different API surface.
- Task plan suggested `with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)` — the 0.31 API for `with_batch_exporter` takes only the exporter (runtime is configured via the `rt-tokio` feature flag on `opentelemetry_sdk`).
- Added a `tracing_opentelemetry_stub` module for the non-telemetry build path so `.with(otel_layer)` compiles when the feature is off. Uses `tracing_subscriber::layer::Identity` as the type.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/telemetry.rs` — Extended with otlp_endpoint field, cfg-gated OTel layer init, TracingGuard shutdown, graceful degradation
- `crates/assay-core/tests/telemetry_otlp.rs` — Updated test_otel_layer_init_compiles from red-state to green-state
