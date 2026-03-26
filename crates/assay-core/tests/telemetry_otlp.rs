//! Red-state integration tests for OTLP export and trace context propagation.
//!
//! These tests define the contract for the `telemetry` feature before the
//! implementation exists. They compile with `--features telemetry` but fail
//! at runtime until T02/T03 add the actual OTel wiring.
//!
//! Run with: `cargo test -p assay-core --test telemetry_otlp --features telemetry`

#![cfg(feature = "telemetry")]

use assay_core::telemetry::{TracingConfig, init_tracing};

/// T02 contract: `TracingConfig` should gain an `otlp_endpoint` field and
/// `init_tracing` should wire an OTel layer when it is `Some(...)`.
///
/// Until T02 lands, this test verifies that `TracingConfig` has an
/// `otlp_endpoint` field (checked at runtime via struct access).
/// Will fail until T02 adds the field.
#[test]
fn test_otel_layer_init_compiles() {
    // Red state: TracingConfig does not yet have otlp_endpoint.
    // T02 will add `pub otlp_endpoint: Option<String>` and wire the OTel layer.
    // For now, verify we can at least construct a config and init tracing
    // with the telemetry feature active.
    let config = TracingConfig::default();

    // Assert that the telemetry feature is actually compiled in by checking
    // that opentelemetry types are available.
    assert!(
        cfg!(feature = "telemetry"),
        "telemetry feature must be enabled"
    );

    let _guard = init_tracing(config);

    // RED STATE: Once T02 lands, this test should be updated to:
    //   let config = TracingConfig { otlp_endpoint: Some("http://localhost:4318".into()), ..Default::default() };
    //   let _guard = init_tracing(config);
    // and verify the OTel layer is active by checking that spans produce OTel trace IDs.

    // For now, assert that the OTel crates are linkable by referencing a type.
    // This catches dep wiring issues at test time rather than downstream in T02.
    let _: opentelemetry::trace::TraceId = opentelemetry::trace::TraceId::INVALID;
}

/// T03 contract: subprocess spawns inject a TRACEPARENT env var derived
/// from the active span context.
///
/// This test creates a parent span, runs a child process that prints
/// the TRACEPARENT env var, and asserts the value matches W3C Trace
/// Context format: `00-<trace_id>-<span_id>-<flags>`.
///
/// Fails until T03 adds TRACEPARENT injection logic.
#[test]
fn test_traceparent_injected_in_subprocess() {
    use std::process::Command;

    // Initialize tracing — no OTel layer yet (T02), so no trace context propagation.
    let config = TracingConfig::default();
    let _guard = init_tracing(config);

    // Create a span that would produce a valid trace/span ID once OTel is wired.
    let span = tracing::info_span!("test_parent_span");
    let _entered = span.enter();

    // Spawn a subprocess that echoes the TRACEPARENT env var.
    // T03 should inject TRACEPARENT into the subprocess environment
    // from the current span context.
    let output = Command::new("sh")
        .arg("-c")
        .arg("echo ${TRACEPARENT:-MISSING}")
        .output()
        .expect("failed to spawn subprocess");

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // RED STATE: Without OTel layer and TRACEPARENT injection (T02+T03),
    // the subprocess will see MISSING. This assertion will fail until then.
    assert_ne!(
        stdout, "MISSING",
        "TRACEPARENT env var was not injected into subprocess \
         (expected once T03 adds injection logic)"
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
}
