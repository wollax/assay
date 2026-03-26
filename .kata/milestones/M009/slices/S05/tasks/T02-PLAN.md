---
estimated_steps: 5
estimated_files: 2
---

# T02: Implement OTel tracing layer in init_tracing() with feature-flagged TracingGuard shutdown

**Slice:** S05 — OTLP export and trace context propagation
**Milestone:** M009

## Description

The core OTel integration. Extends `init_tracing()` to conditionally add an OTLP exporter layer to the subscriber chain when the `telemetry` feature is compiled in and `otlp_endpoint` is configured. Ensures spans are flushed on process exit via TracingGuard::drop(). Implements graceful degradation: OTel init failure produces a visible warning and continues without the OTel layer.

## Steps

1. Add `otlp_endpoint: Option<String>` field to `TracingConfig` with `None` default. This field is always present (not feature-gated) so callers can set it regardless of whether the feature is compiled in.
2. Behind `#[cfg(feature = "telemetry")]` in `init_tracing()`:
   - If `config.otlp_endpoint` is `Some(endpoint)`:
     - Call `opentelemetry::global::set_text_map_propagator(opentelemetry_sdk::propagation::TraceContextPropagator::new())` — must happen before subscriber init
     - Build OTLP exporter: `opentelemetry_otlp::SpanExporter::builder().with_http().with_endpoint(&endpoint).build()`
     - Build tracer provider: `opentelemetry_sdk::trace::SdkTracerProvider::builder().with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio).build()`
     - Set global provider: `opentelemetry::global::set_tracer_provider(provider.clone())`
     - Create tracing-opentelemetry layer: `tracing_opentelemetry::layer().with_tracer(provider.tracer("assay"))`
     - Add `.with(otel_layer)` to the registry chain
   - Wrap the OTel setup in a match/Result — on error, emit `tracing::warn!(endpoint = %endpoint, error = %e, "OTLP exporter init failed; trace export disabled")` and continue with the fmt-only subscriber
3. Behind `#[cfg(feature = "telemetry")]` on `TracingGuard::drop()`:
   - Call `opentelemetry::global::shutdown_tracer_provider()` to flush pending spans
   - This call is synchronous and blocks until flushed
4. Handle the subscriber type divergence: when OTel layer is present vs absent, the registry type differs. Use `Option<Layer>` with `.with(otel_layer_option)` — tracing-subscriber's `.with(Option<L>)` is a no-op when None. This avoids type-gymnastics.
5. Update existing telemetry unit tests: ensure `TracingConfig::default()` has `otlp_endpoint: None` and existing test assertions still hold.

## Must-Haves

- [ ] `TracingConfig.otlp_endpoint: Option<String>` field exists (always, not feature-gated)
- [ ] OTel layer added to subscriber when feature enabled + endpoint configured
- [ ] `set_text_map_propagator(TraceContextPropagator::new())` called before subscriber init
- [ ] OTel init failure emits tracing::warn! with endpoint and error (not silent, not panic)
- [ ] `TracingGuard::drop()` calls `shutdown_tracer_provider()` behind cfg(feature = "telemetry")
- [ ] Existing telemetry unit tests still pass (default config has otlp_endpoint: None)

## Verification

- `cargo test -p assay-core telemetry --features telemetry` — existing + new unit tests pass
- `cargo test -p assay-core --test telemetry_otlp --features telemetry` — OTel init test from T01 now passes
- `cargo build -p assay-cli --features telemetry` — compiles without errors
- `cargo build -p assay-cli` — default build still compiles (otlp_endpoint field is unconditional but OTel code is cfg-gated)

## Observability Impact

- Signals added/changed: `tracing::warn!` on OTel init failure with endpoint and error context; `shutdown_tracer_provider()` on drop ensures span flush
- How a future agent inspects this: `RUST_LOG=opentelemetry=debug` shows BatchSpanProcessor behavior; warn log on init failure is always visible at default log level
- Failure state exposed: Bad endpoint URL → warn log with specific error; missing runtime → warn log; endpoint unreachable → silent span drops (documented: use RUST_LOG=opentelemetry=debug)

## Inputs

- `crates/assay-core/src/telemetry.rs` — existing TracingConfig, TracingGuard, init_tracing() from S01
- T01 output — OTel deps available as optional, feature flags defined
- S05-RESEARCH.md — version alignment (opentelemetry 0.31, tracing-opentelemetry 0.32), transport (http-proto + hyper-client), rt-tokio feature, propagator setup order
- D143 — use rt-tokio with existing runtime

## Expected Output

- `crates/assay-core/src/telemetry.rs` — extended with otlp_endpoint field, cfg-gated OTel layer init, TracingGuard shutdown, graceful degradation
- T01's `test_otel_layer_init_compiles` test now passes
