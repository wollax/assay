//! Contract tests for OTel metrics integration.
//!
//! Defines the API contract for metrics initialization, counter/histogram
//! recording, and feature-flag isolation. These tests will fail to compile
//! until T02 provides the implementation — that is the expected "red state."
//!
//! Run with: `cargo test -p assay-core --test otel_metrics --features telemetry`

#![cfg(feature = "telemetry")]

use opentelemetry_sdk::metrics::SdkMeterProvider;
use serial_test::serial;

/// Construct `SdkMeterProvider` via builder, create a counter, call `.add(1)`,
/// verify no panic. Confirms the `metrics` feature is correctly enabled in
/// workspace deps.
#[test]
#[serial]
fn test_meter_provider_init_compiles() {
    use opentelemetry::metrics::MeterProvider;

    // Build a provider with a simple periodic exporter (no real endpoint needed —
    // we only care that the types resolve and construction doesn't panic).
    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .build()
        .expect("MetricExporter build should succeed");

    let provider = SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .build();

    let meter = provider.meter("assay-test");
    let counter = meter.u64_counter("test_counter").build();
    counter.add(1, &[]);

    // Shutdown cleanly.
    let _ = provider.shutdown();
}

/// Create a histogram via meter, call `.record(42.0)`, verify no panic.
/// Confirms histogram API is available through the `metrics` feature.
#[test]
#[serial]
fn test_histogram_recording() {
    use opentelemetry::metrics::MeterProvider;

    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .build()
        .expect("MetricExporter build should succeed");

    let provider = SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .build();

    let meter = provider.meter("assay-test");
    let histogram = meter.f64_histogram("test_histogram").build();
    histogram.record(42.0, &[]);

    let _ = provider.shutdown();
}

/// Call recording functions without metrics initialization — must not panic.
/// Confirms the OnceLock guard works: when metric handles are not populated,
/// recording is a silent no-op.
///
/// Must run before `test_init_metrics_populates_handles` sets the global
/// OnceLocks (which can never be unset). `#[serial]` ensures exclusivity.
#[test]
#[serial]
fn test_recording_functions_noop_without_init() {
    // These functions should be callable from any context without cfg guards.
    // Before init_metric_handles() is called, they silently no-op.
    assay_core::telemetry::record_session_launched();
    assay_core::telemetry::record_gate_evaluated();
    assay_core::telemetry::record_merge_attempted();
    assay_core::telemetry::record_gate_eval_latency_ms(123.4);
    assay_core::telemetry::record_agent_run_duration_ms(567.8);
    // No panic = pass.
}

/// Call `init_metric_handles` with a real `Meter`, then call each recording
/// function — verify counters increment (no panics, OnceLock populated).
#[test]
#[serial]
fn test_init_metrics_populates_handles() {
    use opentelemetry::metrics::MeterProvider;

    let exporter = opentelemetry_otlp::MetricExporter::builder()
        .with_http()
        .build()
        .expect("MetricExporter build should succeed");

    let provider = SdkMeterProvider::builder()
        .with_periodic_exporter(exporter)
        .build();

    let meter = provider.meter("assay-test");

    // Initialize the global metric handles.
    assay_core::telemetry::init_metric_handles(&meter);

    // All recording functions should now increment real counters/histograms.
    assay_core::telemetry::record_session_launched();
    assay_core::telemetry::record_gate_evaluated();
    assay_core::telemetry::record_merge_attempted();
    assay_core::telemetry::record_gate_eval_latency_ms(15.5);
    assay_core::telemetry::record_agent_run_duration_ms(3200.0);

    // No panic = handles were populated and recording works.
    let _ = provider.shutdown();
}
