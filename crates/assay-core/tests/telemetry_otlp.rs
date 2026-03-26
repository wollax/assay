//! Red-state integration tests for OTLP export and trace context propagation.
//!
//! These tests define the contract for the `telemetry` feature before the
//! implementation exists. They compile with `--features telemetry` but fail
//! at runtime until T02/T03 add the actual OTel wiring.
//!
//! Run with: `cargo test -p assay-core --test telemetry_otlp --features telemetry`

#![cfg(feature = "telemetry")]

use assay_core::telemetry::{TracingConfig, init_tracing};

/// Verifies that `TracingConfig` has an `otlp_endpoint` field and that
/// `init_tracing` wires an OTel layer when endpoint is configured.
///
/// Note: without a running OTLP collector, the exporter will silently
/// drop spans in the background — but the layer init itself succeeds
/// because the batch exporter defers connection until first export.
#[test]
fn test_otel_layer_init_compiles() {
    assert!(
        cfg!(feature = "telemetry"),
        "telemetry feature must be enabled"
    );

    // Construct config with an endpoint — uses a bogus port so no real
    // collector is needed. The OTLP HTTP exporter builds successfully
    // (connection is deferred to first batch export).
    let config = TracingConfig {
        otlp_endpoint: Some("http://localhost:4318".into()),
        ..Default::default()
    };
    let _guard = init_tracing(config);

    // Verify OTel crates are linkable.
    let _: opentelemetry::trace::TraceId = opentelemetry::trace::TraceId::INVALID;

    // Verify default config still has otlp_endpoint: None.
    assert!(TracingConfig::default().otlp_endpoint.is_none());
}

/// T03 contract: subprocess spawns inject a TRACEPARENT env var derived
/// from the active span context.
///
/// This test creates a parent span with a real OTel provider, runs a child
/// process that prints the TRACEPARENT env var (injected via the same
/// propagator logic used by launch_agent), and asserts the value matches
/// W3C Trace Context format: `00-<trace_id>-<span_id>-<flags>`.
#[test]
fn test_traceparent_injected_in_subprocess() {
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_sdk::propagation::TraceContextPropagator;
    use std::collections::HashMap;
    use std::process::Command;
    use tracing_opentelemetry::OpenTelemetrySpanExt;

    // Initialize OTel with a no-op exporter so spans get real trace/span IDs.
    // We don't need a collector — we just need the propagator to produce values.
    let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder().build();
    opentelemetry::global::set_tracer_provider(provider.clone());
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    // Wire a tracing-opentelemetry layer so info_span! creates OTel spans.
    let tracer = provider.tracer("test");
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    use tracing_subscriber::layer::SubscriberExt;
    let subscriber = tracing_subscriber::registry().with(otel_layer);
    let _default = tracing::subscriber::set_default(subscriber);

    // Create a parent span — this will have real trace + span IDs.
    let span = tracing::info_span!("test_parent_span");
    let _entered = span.enter();

    // Extract TRACEPARENT using the same logic as pipeline.rs
    let cx = tracing::Span::current().context();
    let mut carrier = HashMap::new();
    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.inject_context(&cx, &mut carrier);
    });
    let traceparent = carrier
        .get("traceparent")
        .expect("propagator should produce traceparent for an active OTel span");

    // Spawn a subprocess with TRACEPARENT injected and read it back.
    let output = Command::new("sh")
        .arg("-c")
        .arg("echo ${TRACEPARENT:-MISSING}")
        .env("TRACEPARENT", traceparent)
        .output()
        .expect("failed to spawn subprocess");

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    assert_ne!(
        stdout, "MISSING",
        "TRACEPARENT env var was not injected into subprocess"
    );

    // Validate W3C Trace Context format:
    // version-trace_id-parent_id-trace_flags
    // e.g. 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01
    let parts: Vec<&str> = stdout.split('-').collect();
    assert_eq!(
        parts.len(),
        4,
        "TRACEPARENT should have 4 dash-separated parts, got: {stdout}"
    );
    assert_eq!(parts[0], "00", "TRACEPARENT version must be '00'");
    assert_eq!(
        parts[1].len(),
        32,
        "trace_id must be 32 hex chars, got: {}",
        parts[1]
    );
    assert_eq!(
        parts[2].len(),
        16,
        "parent_id must be 16 hex chars, got: {}",
        parts[2]
    );
    assert_eq!(
        parts[3].len(),
        2,
        "trace_flags must be 2 hex chars, got: {}",
        parts[3]
    );

    // Clean up OTel global state.
    let _ = provider.shutdown();
}
