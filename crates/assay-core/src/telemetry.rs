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
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            default_level: "info".to_string(),
            ansi: true,
            with_target: false,
            traces_dir: None,
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

/// Visitor that collects span fields into a `HashMap`.
struct FieldVisitor<'a> {
    fields: &'a mut HashMap<String, serde_json::Value>,
}

impl tracing::field::Visit for FieldVisitor<'_> {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::String(format!("{value:?}")),
        );
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields.insert(
            field.name().to_string(),
            serde_json::Value::String(value.to_string()),
        );
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
        self.fields
            .insert(field.name().to_string(), serde_json::json!(value));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), serde_json::json!(value));
    }
}

/// Generate a trace file ID from the current timestamp + randomness.
///
/// Format: `YYYYMMDDTHHMMSSZ-XXXXXX` (same as history `generate_run_id`).
fn generate_trace_id() -> String {
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos() as u64,
    );
    use std::hash::Hash;
    std::thread::current().id().hash(&mut hasher);
    let suffix = format!("{:06x}", hasher.finish() & 0xFF_FFFF);
    format!("{ts}-{suffix}")
}

/// Custom [`tracing_subscriber::Layer`] that captures span lifecycle events
/// and writes a structured JSON file per trace when the root span closes.
///
/// Thread-safe: uses `Mutex<HashMap<Id, SpanData>>` for span storage.
/// Each root span closure produces one JSON file containing all spans
/// in that trace tree.
pub struct JsonFileLayer {
    traces_dir: PathBuf,
    max_files: usize,
    spans: Mutex<HashMap<u64, SpanData>>,
}

impl JsonFileLayer {
    /// Create a new `JsonFileLayer` that writes trace files to `traces_dir`.
    ///
    /// `max_files` controls the pruning threshold — oldest files are deleted
    /// when the count exceeds this limit.
    pub fn new(traces_dir: PathBuf, max_files: usize) -> Self {
        Self {
            traces_dir,
            max_files,
            spans: Mutex::new(HashMap::new()),
        }
    }

    /// Write the collected spans for a trace to a JSON file atomically.
    fn write_trace_file(&self, spans: Vec<SpanData>) {
        let span_count = spans.len();
        let trace_id = generate_trace_id();
        let filename = format!("{trace_id}.json");
        let filepath = self.traces_dir.join(&filename);

        match serde_json::to_string_pretty(&spans) {
            Ok(json) => match NamedTempFile::new_in(&self.traces_dir) {
                Ok(mut tmpfile) => {
                    use std::io::Write;
                    if let Err(e) = tmpfile.write_all(json.as_bytes()) {
                        tracing::warn!(
                            path = %filepath.display(),
                            error = %e,
                            "failed to write trace file contents"
                        );
                        return;
                    }
                    if let Err(e) = tmpfile.persist(&filepath) {
                        tracing::warn!(
                            path = %filepath.display(),
                            error = %e,
                            "failed to persist trace file"
                        );
                        return;
                    }
                    tracing::debug!(
                        path = %filepath.display(),
                        span_count,
                        "wrote trace file"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        dir = %self.traces_dir.display(),
                        error = %e,
                        "failed to create temp file for trace"
                    );
                }
            },
            Err(e) => {
                tracing::warn!(error = %e, "failed to serialize trace data");
            }
        }

        // Prune old files if count exceeds max_files.
        self.prune_old_files();
    }

    /// Remove oldest trace files when the directory count exceeds `max_files`.
    fn prune_old_files(&self) {
        let entries = match std::fs::read_dir(&self.traces_dir) {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(
                    dir = %self.traces_dir.display(),
                    error = %e,
                    "failed to read traces dir for pruning"
                );
                return;
            }
        };

        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
            .map(|e| e.path())
            .collect();

        if files.len() <= self.max_files {
            return;
        }

        // Sort by filename (which embeds the timestamp) — ascending = oldest first.
        files.sort();

        let to_remove = files.len() - self.max_files;
        for path in files.into_iter().take(to_remove) {
            if let Err(e) = std::fs::remove_file(&path) {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "failed to prune old trace file"
                );
            }
        }
    }

    /// Collect all spans belonging to the same trace tree as `root_id`.
    fn collect_trace_spans(&self, root_id: u64, map: &mut HashMap<u64, SpanData>) -> Vec<SpanData> {
        // Remove the root span.
        let mut trace_spans = Vec::new();
        if let Some(root) = map.remove(&root_id) {
            trace_spans.push(root);
        }

        // Collect all spans whose parent chain leads to root_id.
        let child_ids: Vec<u64> = map
            .iter()
            .filter(|(_, data)| {
                let mut parent = data.parent_id;
                while let Some(pid) = parent {
                    if pid == root_id {
                        return true;
                    }
                    parent = map.get(&pid).and_then(|p| p.parent_id);
                }
                false
            })
            .map(|(id, _)| *id)
            .collect();

        for id in child_ids {
            if let Some(span) = map.remove(&id) {
                trace_spans.push(span);
            }
        }

        trace_spans
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
        let metadata = attrs.metadata();
        let parent_id = attrs
            .parent()
            .map(|p| p.into_non_zero_u64().get())
            .or_else(|| {
                if attrs.is_contextual() {
                    ctx.current_span()
                        .id()
                        .map(|id| id.into_non_zero_u64().get())
                } else {
                    None
                }
            });

        let mut fields = HashMap::new();
        let mut visitor = FieldVisitor {
            fields: &mut fields,
        };
        attrs.record(&mut visitor);

        let span_data = SpanData {
            name: metadata.name().to_string(),
            target: metadata.target().to_string(),
            level: format!("{}", metadata.level()),
            span_id: id.into_non_zero_u64().get(),
            parent_id,
            start_time: chrono::Utc::now().to_rfc3339(),
            end_time: None,
            duration_ms: None,
            fields,
        };

        if let Ok(mut map) = self.spans.lock() {
            map.insert(id.into_non_zero_u64().get(), span_data);
        }
    }

    fn on_record(
        &self,
        id: &tracing::span::Id,
        values: &tracing::span::Record<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Ok(mut map) = self.spans.lock()
            && let Some(span_data) = map.get_mut(&id.into_non_zero_u64().get())
        {
            let mut visitor = FieldVisitor {
                fields: &mut span_data.fields,
            };
            values.record(&mut visitor);
        }
    }

    fn on_close(&self, id: tracing::span::Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let numeric_id = id.into_non_zero_u64().get();
        let now = chrono::Utc::now();

        let mut map = match self.spans.lock() {
            Ok(m) => m,
            Err(_) => return,
        };

        // Update end time and duration for the closing span.
        if let Some(span_data) = map.get_mut(&numeric_id) {
            let end_str = now.to_rfc3339();
            // Parse start_time to compute duration.
            if let Ok(start) = chrono::DateTime::parse_from_rfc3339(&span_data.start_time) {
                let duration = now.signed_duration_since(start);
                span_data.duration_ms = Some(duration.num_milliseconds() as f64);
            }
            span_data.end_time = Some(end_str);
        }

        // Check if this is a root span (no parent) — if so, flush the trace.
        let is_root = map.get(&numeric_id).is_some_and(|s| s.parent_id.is_none());

        if is_root {
            let trace_spans = self.collect_trace_spans(numeric_id, &mut map);
            // Drop lock before I/O.
            drop(map);
            if !trace_spans.is_empty() {
                self.write_trace_file(trace_spans);
            }
        }
    }
}

/// RAII guard that flushes the non-blocking writer on drop.
///
/// **Hold this for the lifetime of the program.** Dropping it early may lose
/// buffered log events.
pub struct TracingGuard {
    _worker_guard: WorkerGuard,
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
/// - When `traces_dir` is `Some`, adds a [`JsonFileLayer`] that writes
///   one JSON file per trace to the specified directory.
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

    // Build optional JSON file layer.
    let json_layer = config.traces_dir.map(|dir| {
        // Create the traces directory if it does not exist.
        if let Err(e) = std::fs::create_dir_all(&dir) {
            eprintln!(
                "[assay] warning: failed to create traces dir '{}': {e}",
                dir.display()
            );
        }
        JsonFileLayer::new(dir, 50)
    });

    // try_init: skips installation if a subscriber is already set — no panic,
    // but a background writer thread is still allocated until the guard drops.
    if let Err(_e) = tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(json_layer)
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
    }
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
    }

    #[test]
    fn test_mcp_config() {
        let cfg = TracingConfig::mcp();
        assert_eq!(cfg.default_level, "warn");
        assert!(!cfg.ansi);
        assert!(!cfg.with_target);
        assert!(cfg.traces_dir.is_none());
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
