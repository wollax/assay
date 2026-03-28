//! Centralized tracing subscriber initialization.
//!
//! All binaries (CLI, TUI, MCP) call [`init_tracing`] once at startup to
//! establish a layered `fmt` subscriber writing structured events to stderr.
//!
//! The returned [`TracingGuard`] **must** be held alive for the lifetime of
//! the program — dropping it flushes the non-blocking writer.
//!
//! # Architecture
//!
//! The subscriber is built from composable layers so that future work (S04/S05)
//! can add JSON file logging or OTLP export without changing call sites.
//!
//! # Examples
//!
//! ```no_run
//! use assay_core::telemetry::{TracingConfig, init_tracing};
//!
//! let _guard = init_tracing(TracingConfig::default());
//! tracing::info!("subscriber is active");
//! ```

use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher, RandomState};
use std::path::PathBuf;
use std::sync::Mutex;
#[cfg(feature = "telemetry")]
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Configuration for [`init_tracing`].
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Default log level directive when `RUST_LOG` is unset or invalid.
    ///
    /// Accepts any [`EnvFilter`]-compatible string (e.g. `"info"`,
    /// `"assay_core=debug,warn"`).
    pub default_level: String,

    /// Enable ANSI color codes in output.
    ///
    /// Set to `false` for machine-readable contexts (MCP, CI, file output).
    pub ansi: bool,

    /// Include the event's target module path in output.
    pub with_target: bool,

    /// Optional directory for JSON trace file export.
    ///
    /// When `Some`, a [`JsonFileLayer`] is added to the subscriber that writes
    /// one JSON file per trace (rooted span tree) into this directory.
    /// Set to `None` (the default) to disable trace file export.
    pub traces_dir: Option<PathBuf>,

    /// OTLP exporter endpoint URL (e.g. `"http://localhost:4318"`).
    ///
    /// When `Some` and the `telemetry` feature is compiled in, an
    /// OpenTelemetry OTLP exporter layer is added to the subscriber chain.
    /// When `None` or when the `telemetry` feature is not enabled, no OTel
    /// layer is added and there is zero overhead.
    pub otlp_endpoint: Option<String>,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            default_level: "info".to_string(),
            ansi: true,
            with_target: false,
            traces_dir: None,
            otlp_endpoint: None,
        }
    }
}

impl TracingConfig {
    /// Preset for MCP server operation.
    ///
    /// Default level `warn`, ANSI disabled (stdout reserved for JSON-RPC).
    pub fn mcp() -> Self {
        Self {
            default_level: "warn".to_string(),
            ansi: false,
            with_target: false,
            traces_dir: None,
            otlp_endpoint: None,
        }
    }
}

// ---------------------------------------------------------------------------
// JSON file trace export layer
// ---------------------------------------------------------------------------

/// A single span's captured data for JSON trace export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanData {
    /// Span name (from instrumentation).
    pub name: String,
    /// Module target path.
    pub target: String,
    /// Tracing level (e.g. `"INFO"`, `"DEBUG"`).
    pub level: String,
    /// Numeric span ID (serialized from `tracing::span::Id`).
    pub span_id: u64,
    /// Parent span ID, or `None` for root spans.
    pub parent_id: Option<u64>,
    /// RFC 3339 start timestamp.
    pub start_time: String,
    /// RFC 3339 end timestamp (populated on close).
    pub end_time: Option<String>,
    /// Duration in milliseconds (populated on close).
    pub duration_ms: Option<f64>,
    /// Recorded key-value fields.
    pub fields: HashMap<String, serde_json::Value>,
}

/// Visitor that collects tracing event fields into a `HashMap<String, String>`.
struct FieldCollector(HashMap<String, serde_json::Value>);

impl tracing::field::Visit for FieldCollector {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.0.insert(
            field.name().to_string(),
            serde_json::Value::String(format!("{value:?}")),
        );
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.0.insert(
            field.name().to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.0
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.0
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.0
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.0
            .insert(field.name().to_string(), serde_json::json!(value));
    }
}

/// A `tracing_subscriber` layer that writes one JSON file per trace to a
/// configured directory.
///
/// Thread-safe: uses `Mutex<HashMap<Id, SpanData>>` for span storage.
pub struct JsonFileLayer {
    traces_dir: PathBuf,
    max_files: usize,
    spans: Mutex<HashMap<u64, SpanData>>,
}

impl JsonFileLayer {
    /// Create a new `JsonFileLayer` that writes trace files to `traces_dir`.
    ///
    /// Old files are pruned when the count exceeds `max_files`.
    pub fn new(traces_dir: PathBuf, max_files: usize) -> Self {
        Self {
            traces_dir,
            max_files,
            spans: Mutex::new(HashMap::new()),
        }
    }

    fn write_trace_file(&self, spans: Vec<SpanData>) {
        let root = spans.iter().find(|s| s.parent_id.is_none()).unwrap();
        let timestamp = root.start_time.clone();
        // Use a random suffix for uniqueness within the same millisecond.
        let suffix = {
            let mut h = RandomState::new().build_hasher();
            h.write_u64(root.span_id);
            format!("{:016x}", h.finish())
        };
        let filename = format!(
            "{timestamp}-{suffix}.json",
            timestamp = &timestamp[..23].replace([':', '.'], "-")
        );
        let filepath = self.traces_dir.join(&filename);

        let span_count = spans.len();
        match serde_json::to_string_pretty(&spans) {
            Ok(json) => match NamedTempFile::new_in(&self.traces_dir) {
                Ok(mut tmp) => {
                    use std::io::Write as _;
                    if let Err(e) = tmp.write_all(json.as_bytes()) {
                        tracing::warn!(path = %filepath.display(), error = %e, "trace file write failed");
                        return;
                    }
                    match tmp.persist(&filepath) {
                        Ok(_) => {
                            tracing::debug!(path = %filepath.display(), span_count, "trace file written");
                            self.prune_old_files();
                        }
                        Err(e) => {
                            tracing::warn!(path = %filepath.display(), error = %e, "trace file persist failed");
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        dir = %self.traces_dir.display(),
                        error = %e,
                        "trace file temp-create failed"
                    );
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "trace file JSON serialization failed");
            }
        }
    }

    fn prune_old_files(&self) {
        let entries = match std::fs::read_dir(&self.traces_dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(dir = %self.traces_dir.display(), error = %e, "trace prune read_dir failed");
                return;
            }
        };

        let mut files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
            .collect();

        if files.len() <= self.max_files {
            return;
        }

        files.sort_by_key(|e| e.file_name());
        let to_delete = files.len() - self.max_files;
        for entry in files.into_iter().take(to_delete) {
            if let Err(e) = std::fs::remove_file(entry.path()) {
                tracing::warn!(path = %entry.path().display(), error = %e, "trace prune delete failed");
            }
        }
    }
}

impl<S> tracing_subscriber::Layer<S> for JsonFileLayer
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let current_span = ctx.current_span();
        let parent_id = attrs
            .parent()
            .or_else(|| current_span.id())
            .map(|p| p.into_u64());

        let mut fields = FieldCollector(HashMap::new());
        attrs.values().record(&mut fields);

        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let data = SpanData {
            name: attrs.metadata().name().to_string(),
            target: attrs.metadata().target().to_string(),
            level: attrs.metadata().level().to_string(),
            span_id: id.into_u64(),
            parent_id,
            start_time: now,
            end_time: None,
            duration_ms: None,
            fields: fields.0,
        };

        if let Ok(mut guard) = self.spans.lock() {
            guard.insert(id.into_u64(), data);
        }
    }

    fn on_record(
        &self,
        id: &tracing::span::Id,
        values: &tracing::span::Record<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut collector = FieldCollector(HashMap::new());
        values.record(&mut collector);

        if let Ok(mut guard) = self.spans.lock()
            && let Some(span) = guard.get_mut(&id.into_u64())
        {
            span.fields.extend(collector.0);
        }
    }

    fn on_close(&self, id: tracing::span::Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        let span_id_u64 = id.into_u64();

        // Update end_time and duration, check if root.
        let (is_root, parent_id) = if let Ok(mut guard) = self.spans.lock() {
            if let Some(span) = guard.get_mut(&span_id_u64) {
                span.end_time = Some(now);
                let is_root = span.parent_id.is_none();
                if let Ok(start) = chrono::DateTime::parse_from_rfc3339(&span.start_time)
                    && let Some(end) = span.end_time.as_deref()
                    && let Ok(end_dt) = chrono::DateTime::parse_from_rfc3339(end)
                {
                    let dur = end_dt.signed_duration_since(start);
                    span.duration_ms = Some(dur.num_milliseconds() as f64);
                }
                (is_root, span.parent_id)
            } else {
                (false, None)
            }
        } else {
            // Recover from a poisoned mutex — best-effort telemetry should not
            // crash the application.
            (false, None)
        };

        let _ = parent_id; // parent_id used implicitly for root detection

        if is_root {
            // Collect the entire trace tree and flush it to a file.
            let trace_spans: Vec<SpanData> = if let Ok(mut guard) = self.spans.lock() {
                // Collect all spans that belong to this root (the root itself plus descendants).
                // Since parent-child relationships are tracked via parent_id, we do a full drain
                // and collect all spans (for simplicity; a single root trace is the common case).
                let all: Vec<SpanData> = guard.drain().map(|(_, v)| v).collect();
                // Sort by start_time for readable output.
                let mut sorted = all;
                sorted.sort_by(|a, b| a.start_time.cmp(&b.start_time));
                sorted
            } else {
                return;
            };

            if !trace_spans.is_empty() {
                self.write_trace_file(trace_spans);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// OTel metrics infrastructure
// ---------------------------------------------------------------------------

/// Build an OTel `SdkMeterProvider` for metrics export.
///
/// Returns `Some(provider)` when `config.otlp_endpoint` is set and the
/// exporter builds successfully. Returns `None` silently when no endpoint
/// is configured; returns `None` with a `tracing::warn!` on initialization
/// failure.
#[cfg(feature = "telemetry")]
fn build_otel_metrics(
    config: &TracingConfig,
) -> Option<opentelemetry_sdk::metrics::SdkMeterProvider> {
    use opentelemetry_otlp::WithExportConfig;

    let endpoint = config.otlp_endpoint.as_deref()?;

    let result =
        (|| -> Result<opentelemetry_sdk::metrics::SdkMeterProvider, Box<dyn std::error::Error>> {
            let exporter = opentelemetry_otlp::MetricExporter::builder()
                .with_http()
                .with_endpoint(endpoint)
                .build()?;

            let provider = opentelemetry_sdk::metrics::SdkMeterProvider::builder()
                .with_periodic_exporter(exporter)
                .build();

            Ok(provider)
        })();

    match result {
        Ok(provider) => {
            tracing::debug!(
                endpoint = %endpoint,
                "OTel metrics provider initialized successfully"
            );
            Some(provider)
        }
        Err(e) => {
            tracing::warn!(
                endpoint = %endpoint,
                error = %e,
                "OTel metrics provider init failed; metrics export disabled. \
                 Verify the endpoint is reachable and a collector is running. \
                 Set RUST_LOG=opentelemetry=debug for details."
            );
            None
        }
    }
}

// Five global metric handles behind OnceLock (telemetry feature only).
#[cfg(feature = "telemetry")]
static SESSIONS_LAUNCHED: OnceLock<opentelemetry::metrics::Counter<u64>> = OnceLock::new();
#[cfg(feature = "telemetry")]
static GATES_EVALUATED: OnceLock<opentelemetry::metrics::Counter<u64>> = OnceLock::new();
#[cfg(feature = "telemetry")]
static MERGES_ATTEMPTED: OnceLock<opentelemetry::metrics::Counter<u64>> = OnceLock::new();
#[cfg(feature = "telemetry")]
static GATE_EVAL_LATENCY: OnceLock<opentelemetry::metrics::Histogram<f64>> = OnceLock::new();
#[cfg(feature = "telemetry")]
static AGENT_RUN_DURATION: OnceLock<opentelemetry::metrics::Histogram<f64>> = OnceLock::new();

/// Initialize the five global metric handles from a `Meter`.
///
/// Must be called once after the `SdkMeterProvider` is built. If called
/// multiple times (e.g. in tests), the second call's instruments are
/// silently dropped; the handles remain bound to the first provider's
/// `Meter`. A `tracing::warn!` is emitted for each handle that was
/// already initialized.
#[cfg(feature = "telemetry")]
pub fn init_metric_handles(meter: &opentelemetry::metrics::Meter) {
    let sets: [(&str, bool); 5] = [
        (
            "SESSIONS_LAUNCHED",
            SESSIONS_LAUNCHED
                .set(meter.u64_counter("assay.sessions.launched").build())
                .is_ok(),
        ),
        (
            "GATES_EVALUATED",
            GATES_EVALUATED
                .set(meter.u64_counter("assay.gates.evaluated").build())
                .is_ok(),
        ),
        (
            "MERGES_ATTEMPTED",
            MERGES_ATTEMPTED
                .set(meter.u64_counter("assay.merges.attempted").build())
                .is_ok(),
        ),
        (
            "GATE_EVAL_LATENCY",
            GATE_EVAL_LATENCY
                .set(meter.f64_histogram("assay.gate_eval.latency_ms").build())
                .is_ok(),
        ),
        (
            "AGENT_RUN_DURATION",
            AGENT_RUN_DURATION
                .set(meter.f64_histogram("assay.agent_run.duration_ms").build())
                .is_ok(),
        ),
    ];
    for (name, was_new) in sets {
        if !was_new {
            tracing::warn!(
                handle = name,
                "OTel metric handle already initialized; second provider's \
                 meter will not be used — recording functions retain the first handle"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Thin recording functions — callable without cfg guards
// ---------------------------------------------------------------------------

/// Record a session launch event. No-op when metrics are not initialized.
#[cfg(feature = "telemetry")]
pub fn record_session_launched() {
    if let Some(c) = SESSIONS_LAUNCHED.get() {
        c.add(1, &[]);
    }
}

/// Record a session launch event. No-op stub when telemetry feature is off.
#[cfg(not(feature = "telemetry"))]
pub fn record_session_launched() {}

/// Record a gate evaluation event. No-op when metrics are not initialized.
#[cfg(feature = "telemetry")]
pub fn record_gate_evaluated() {
    if let Some(c) = GATES_EVALUATED.get() {
        c.add(1, &[]);
    }
}

/// Record a gate evaluation event. No-op stub when telemetry feature is off.
#[cfg(not(feature = "telemetry"))]
pub fn record_gate_evaluated() {}

/// Record a merge attempt event. No-op when metrics are not initialized.
#[cfg(feature = "telemetry")]
pub fn record_merge_attempted() {
    if let Some(c) = MERGES_ATTEMPTED.get() {
        c.add(1, &[]);
    }
}

/// Record a merge attempt event. No-op stub when telemetry feature is off.
#[cfg(not(feature = "telemetry"))]
pub fn record_merge_attempted() {}

/// Record gate evaluation latency in milliseconds. No-op when metrics are not initialized.
#[cfg(feature = "telemetry")]
pub fn record_gate_eval_latency_ms(ms: f64) {
    if let Some(h) = GATE_EVAL_LATENCY.get() {
        h.record(ms, &[]);
    }
}

/// Record gate evaluation latency in milliseconds. No-op stub when telemetry feature is off.
#[cfg(not(feature = "telemetry"))]
pub fn record_gate_eval_latency_ms(_ms: f64) {}

/// Record agent run duration in milliseconds. No-op when metrics are not initialized.
#[cfg(feature = "telemetry")]
pub fn record_agent_run_duration_ms(ms: f64) {
    if let Some(h) = AGENT_RUN_DURATION.get() {
        h.record(ms, &[]);
    }
}

/// Record agent run duration in milliseconds. No-op stub when telemetry feature is off.
#[cfg(not(feature = "telemetry"))]
pub fn record_agent_run_duration_ms(_ms: f64) {}

// ---------------------------------------------------------------------------
// TracingGuard
// ---------------------------------------------------------------------------

/// RAII guard that flushes the non-blocking writer on drop.
///
/// **Hold this for the lifetime of the program.** Dropping it early may lose
/// buffered log events.
///
/// When the `telemetry` feature is enabled and an OTLP exporter was
/// initialized, dropping the guard shuts down the meter provider (flushing
/// pending metrics) and then the tracer provider (flushing pending spans),
/// in that order (D179).
pub struct TracingGuard {
    _worker_guard: WorkerGuard,
    #[cfg(feature = "telemetry")]
    _meter_provider: Option<opentelemetry_sdk::metrics::SdkMeterProvider>,
    #[cfg(feature = "telemetry")]
    _tracer_provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
}

#[cfg(feature = "telemetry")]
impl Drop for TracingGuard {
    fn drop(&mut self) {
        // D179: Shut down meter provider FIRST, then tracer provider.
        // Metrics may reference trace context; flushing metrics first ensures
        // no dangling references during tracer shutdown.
        if let Some(ref provider) = self._meter_provider
            && let Err(e) = provider.shutdown()
        {
            eprintln!(
                "[assay] ERROR: OTel meter provider failed to flush pending metrics on shutdown: {e}. \
                 Some metrics may not have been exported to the collector."
            );
        }

        if let Some(ref provider) = self._tracer_provider
            && let Err(e) = provider.shutdown()
        {
            eprintln!(
                "[assay] ERROR: OTel tracer provider failed to flush pending spans on shutdown: {e}. \
                 Some traces may not have been exported to the collector."
            );
        }
    }
}

// ---------------------------------------------------------------------------
// init_tracing
// ---------------------------------------------------------------------------

/// Initialize the global tracing subscriber.
///
/// Builds a layered `fmt` subscriber that:
/// - Writes to stderr via a non-blocking writer
/// - Respects `RUST_LOG` for filtering, falling back to
///   [`TracingConfig::default_level`] on parse error (with a stderr warning)
/// - Uses [`try_init`](tracing_subscriber::util::SubscriberInitExt::try_init)
///   so calling this a second time does not panic; the second subscriber
///   installation is silently skipped, though a background writer thread
///   is still spawned and held until the returned guard drops.
///
/// When `traces_dir` is set in the config, a [`JsonFileLayer`] is added that
/// writes one JSON file per root span to the directory.
///
/// When the `telemetry` feature is enabled and
/// [`TracingConfig::otlp_endpoint`] is `Some`, an OTLP exporter layer is
/// added to the subscriber chain. If OTel initialization fails, a warning
/// is emitted and the subscriber continues with the fmt layer only.
///
/// Returns a [`TracingGuard`] whose [`WorkerGuard`] flushes on drop.
pub fn init_tracing(mut config: TracingConfig) -> TracingGuard {
    let filter = match EnvFilter::try_from_default_env() {
        Ok(f) => f,
        Err(e) => {
            // Emit to stderr directly — the subscriber is not yet established.
            if std::env::var_os("RUST_LOG").is_some() {
                eprintln!(
                    "[assay] warning: RUST_LOG is invalid ({e}); falling back to '{}'",
                    config.default_level
                );
            }
            EnvFilter::new(&config.default_level)
        }
    };

    let (non_blocking, worker_guard) = tracing_appender::non_blocking(std::io::stderr());

    let fmt_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(config.ansi)
        .with_target(config.with_target);

    // Take traces_dir before lending config to build_otel_layer.
    let traces_dir = config.traces_dir.take();

    // Build optional JSON file layer.
    // Use `and_then` so a directory-creation failure disables the layer entirely
    // rather than registering it and flooding the log with per-span I/O errors.
    let json_layer = traces_dir.and_then(|dir| {
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!(
                "[assay] warning: failed to create traces dir '{}': {e}; trace file export disabled",
                dir.display()
            );
            return None;
        }
        Some(JsonFileLayer::new(dir, 50))
    });

    // Build the optional OTel layer behind cfg(feature = "telemetry").
    #[cfg(feature = "telemetry")]
    let (otel_layer, tracer_provider) = build_otel_layer(&config);

    // Build the optional OTel metrics provider behind cfg(feature = "telemetry").
    #[cfg(feature = "telemetry")]
    let meter_provider = build_otel_metrics(&config);

    // If meter provider was built, initialize global metric handles.
    #[cfg(feature = "telemetry")]
    if let Some(ref provider) = meter_provider {
        use opentelemetry::metrics::MeterProvider;
        let meter = provider.meter("assay");
        init_metric_handles(&meter);
    }

    // When telemetry feature is not compiled, provide a None placeholder so
    // the `.with(otel_layer)` call is a type-safe no-op.
    #[cfg(not(feature = "telemetry"))]
    let otel_layer: Option<tracing_opentelemetry_stub::NoLayer> = None;

    // try_init: skips installation if a subscriber is already set — no panic,
    // but a background writer thread is still allocated until the guard drops.
    if let Err(_e) = tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(json_layer)
        .with(otel_layer)
        .try_init()
    {
        eprintln!(
            "[assay] warning: tracing subscriber already initialized; \
             this init (default_level='{}') was skipped — first init wins.",
            config.default_level
        );
    }

    TracingGuard {
        _worker_guard: worker_guard,
        #[cfg(feature = "telemetry")]
        _meter_provider: meter_provider,
        #[cfg(feature = "telemetry")]
        _tracer_provider: tracer_provider,
    }
}

/// Build the OTel tracing layer when the `telemetry` feature is enabled.
///
/// Returns `(Some(layer), Some(provider))` on success, or `(None, None)` when
/// `otlp_endpoint` is `None` or initialization fails (with a warning logged).
#[cfg(feature = "telemetry")]
fn build_otel_layer<S>(
    config: &TracingConfig,
) -> (
    Option<tracing_opentelemetry::OpenTelemetryLayer<S, opentelemetry_sdk::trace::Tracer>>,
    Option<opentelemetry_sdk::trace::SdkTracerProvider>,
)
where
    S: tracing::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    use opentelemetry::trace::TracerProvider;
    use opentelemetry_otlp::{SpanExporter, WithExportConfig};

    let endpoint = match config.otlp_endpoint.as_deref() {
        Some(ep) => ep,
        None => return (None, None),
    };

    // Build OTLP HTTP exporter → tracer provider → tracing layer.
    // The global propagator and provider are only set on success — this
    // prevents a partial-init state where extract_traceparent() would inject
    // an empty/zeroed TRACEPARENT if the exporter build fails.
    let result = (|| -> Result<
        (
            tracing_opentelemetry::OpenTelemetryLayer<S, opentelemetry_sdk::trace::Tracer>,
            opentelemetry_sdk::trace::SdkTracerProvider,
        ),
        Box<dyn std::error::Error>,
    > {
        let exporter = SpanExporter::builder()
            .with_http()
            .with_endpoint(endpoint)
            .build()?;

        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .build();

        // Set globals only after both exporter and provider are successfully
        // constructed, so callers never see a propagator without a live provider.
        opentelemetry::global::set_text_map_propagator(
            opentelemetry_sdk::propagation::TraceContextPropagator::new(),
        );
        opentelemetry::global::set_tracer_provider(provider.clone());

        let tracer = provider.tracer("assay");
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        Ok((otel_layer, provider))
    })();

    match result {
        Ok((layer, provider)) => (Some(layer), Some(provider)),
        Err(e) => {
            // Graceful degradation: warn and continue without OTel.
            tracing::warn!(
                endpoint = %endpoint,
                error = %e,
                "OTLP exporter init failed; trace export disabled. \
                 Verify OTEL_EXPORTER_OTLP_ENDPOINT is reachable \
                 and a collector is running. \
                 Set RUST_LOG=opentelemetry=debug for details."
            );
            (None, None)
        }
    }
}

/// Stub module so `Option<NoLayer>` compiles when telemetry feature is off.
#[cfg(not(feature = "telemetry"))]
mod tracing_opentelemetry_stub {
    pub type NoLayer = tracing_subscriber::layer::Identity;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let cfg = TracingConfig::default();
        assert_eq!(cfg.default_level, "info");
        assert!(cfg.ansi);
        assert!(!cfg.with_target);
        assert!(cfg.traces_dir.is_none());
        assert!(cfg.otlp_endpoint.is_none());
    }

    #[test]
    fn test_mcp_config() {
        let cfg = TracingConfig::mcp();
        assert_eq!(cfg.default_level, "warn");
        assert!(!cfg.ansi);
        assert!(!cfg.with_target);
        assert!(cfg.traces_dir.is_none());
        assert!(cfg.otlp_endpoint.is_none());
    }

    #[test]
    fn test_init_tracing_returns_guard() {
        // This test verifies that init_tracing completes without panic and
        // returns a guard. Because try_init is used, this is safe even if
        // another test in the process already initialized a subscriber.
        let _guard = init_tracing(TracingConfig::default());
        // Guard holds a WorkerGuard. If a subscriber was already registered,
        // the writer is unused but safely cleaned up when the guard drops.
    }

    #[test]
    fn test_double_init_is_safe() {
        // Second call must not panic — try_init silently skips subscriber
        // installation. The returned guard is still valid and droppable.
        let _guard1 = init_tracing(TracingConfig::default());
        let _guard2 = init_tracing(TracingConfig::mcp());
    }
}
