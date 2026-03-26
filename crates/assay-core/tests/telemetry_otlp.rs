//! Integration tests for OTLP export and trace context propagation.
//!
//! Verifies the `telemetry` feature flag contract:
//! - `init_tracing` accepts an OTLP endpoint and does not panic.
//! - Subprocess spawns receive a valid W3C TRACEPARENT env var derived from
//!   the active span context.
//! - `inject_traceparent` directly injects the env var into a `Command`.
//! - Default build has zero OTel deps.
//!
//! Run with: `cargo test -p assay-core --test telemetry_otlp --features telemetry`
//!
//! Tests that mutate OTel globals (set_tracer_provider, set_text_map_propagator)
//! are serialized via `serial_test` to prevent interference in parallel runs.

#![cfg(feature = "telemetry")]

use assay_core::telemetry::{TracingConfig, init_tracing};
use serial_test::serial;

/// Verifies that:
/// - `TracingConfig` has an `otlp_endpoint` field.
/// - `init_tracing` with a configured endpoint does not panic.
/// - OTel crate types are linkable under the `telemetry` feature.
///
/// Note: This test cannot verify the OTel layer is actually added to the
/// subscriber chain because the batch exporter defers connection — whether
/// init succeeded or gracefully degraded, `init_tracing` returns normally.
/// Real end-to-end verification requires a live collector (see S05-UAT.md).
#[test]
#[serial]
fn test_otel_layer_init_compiles() {
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

/// Verifies that subprocess spawns receive a valid W3C TRACEPARENT env var.
///
/// Creates a parent span with a real OTel provider+propagator, calls
/// `inject_traceparent` (the actual production helper from pipeline.rs),
/// then reads the env var back from a child process.
///
/// W3C Trace Context format: `00-<32hex trace_id>-<16hex parent_id>-<2hex flags>`
/// Both trace_id and parent_id must be non-zero (non-invalid span context).
#[test]
#[serial]
fn test_traceparent_injected_in_subprocess() {
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_sdk::propagation::TraceContextPropagator;
    use std::process::Command;

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

    // Build the subprocess command using the actual inject_traceparent helper
    // from pipeline.rs via its test-visible wrapper. This validates that the
    // production injection path — not a manually re-implemented version — works.
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg("echo ${TRACEPARENT:-MISSING}");
    assay_core::pipeline::inject_traceparent_for_test(&mut cmd);

    let output = cmd.output().expect("failed to spawn subprocess");
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

    // Validate values are non-zero — a zeroed trace_id/parent_id indicates
    // a broken or sampled-out span context that collectors would reject.
    assert_ne!(
        parts[1], "00000000000000000000000000000000",
        "trace_id must not be all zeros (invalid span context)"
    );
    assert_ne!(
        parts[2], "0000000000000000",
        "parent_id must not be all zeros (invalid span context)"
    );

    // Clean up OTel global state.
    let _ = provider.shutdown();
}

/// Verifies that `extract_traceparent` returns None when called outside any span.
///
/// The no-span guard in `extract_traceparent` is a safety rail: if broken,
/// code outside any span could inject a garbage/zeroed TRACEPARENT into
/// subprocesses.
#[test]
#[serial]
fn test_extract_traceparent_returns_none_outside_span() {
    use opentelemetry_sdk::propagation::TraceContextPropagator;

    // Set up a propagator so extract_traceparent has something to call.
    opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());

    // No span entered — use a bare registry (no OTel layer) so Span::current()
    // is disabled.
    let sub = tracing_subscriber::registry();
    let _default = tracing::subscriber::set_default(sub);

    // Call the test-visible wrapper; must return None, not Some.
    let tp = assay_core::pipeline::extract_traceparent_for_test();
    assert!(
        tp.is_none(),
        "TRACEPARENT must not be injected outside any span, got: {tp:?}"
    );
}
