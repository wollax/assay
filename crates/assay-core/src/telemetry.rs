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

use tracing_appender::non_blocking::WorkerGuard;
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
            otlp_endpoint: None,
        }
    }
}

/// RAII guard that flushes the non-blocking writer on drop.
///
/// **Hold this for the lifetime of the program.** Dropping it early may lose
/// buffered log events.
///
/// When the `telemetry` feature is enabled and an OTLP exporter was
/// initialized, dropping the guard also shuts down the tracer provider,
/// flushing any pending spans.
pub struct TracingGuard {
    _worker_guard: WorkerGuard,
    #[cfg(feature = "telemetry")]
    _tracer_provider: Option<opentelemetry_sdk::trace::SdkTracerProvider>,
}

#[cfg(feature = "telemetry")]
impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Some(ref provider) = self._tracer_provider
            && let Err(e) = provider.shutdown()
        {
            eprintln!("[assay] warning: OTel tracer provider shutdown error: {e}");
        }
    }
}

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
/// When the `telemetry` feature is enabled and
/// [`TracingConfig::otlp_endpoint`] is `Some`, an OTLP exporter layer is
/// added to the subscriber chain. If OTel initialization fails, a warning
/// is emitted and the subscriber continues with the fmt layer only.
///
/// Returns a [`TracingGuard`] whose [`WorkerGuard`] flushes on drop.
pub fn init_tracing(config: TracingConfig) -> TracingGuard {
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

    // Build the optional OTel layer behind cfg(feature = "telemetry").
    #[cfg(feature = "telemetry")]
    let (otel_layer, tracer_provider) = build_otel_layer(&config);

    // When telemetry feature is not compiled, provide a None placeholder so
    // the `.with(otel_layer)` call is a type-safe no-op.
    #[cfg(not(feature = "telemetry"))]
    let otel_layer: Option<tracing_opentelemetry_stub::NoLayer> = None;

    // try_init: skips installation if a subscriber is already set — no panic,
    // but a background writer thread is still allocated until the guard drops.
    //
    // `.with(Option<L>)` is a no-op when None, avoiding type divergence.
    if let Err(_e) = tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
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

    // Propagator must be set before subscriber init so incoming trace context
    // is extracted correctly from the very first span.
    opentelemetry::global::set_text_map_propagator(
        opentelemetry_sdk::propagation::TraceContextPropagator::new(),
    );

    // Build OTLP HTTP exporter → tracer provider → tracing layer.
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
                "OTLP exporter init failed; trace export disabled"
            );
            (None, None)
        }
    }
}

/// Stub module so `Option<NoLayer>` compiles when telemetry feature is off.
/// `tracing_subscriber::Layer` is implemented for `Option<L>` where L: Layer,
/// so we need a concrete type. We use `tracing_subscriber::layer::Identity`
/// which is always available and acts as a passthrough.
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
        assert!(cfg.otlp_endpoint.is_none());
    }

    #[test]
    fn test_mcp_config() {
        let cfg = TracingConfig::mcp();
        assert_eq!(cfg.default_level, "warn");
        assert!(!cfg.ansi);
        assert!(!cfg.with_target);
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
