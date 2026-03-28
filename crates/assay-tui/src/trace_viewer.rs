//! Trace viewer data types and logic for the TUI trace viewer screen.
//!
//! Provides [`TraceEntry`] (parsed trace file metadata), [`SpanLine`] (flattened
//! span tree line for rendering), and the core functions [`load_traces`],
//! [`load_trace_spans`], and [`flatten_span_tree`].

use std::collections::HashMap;
use std::path::Path;

use assay_core::telemetry::SpanData;

/// Metadata for a single trace file, displayed in the trace list.
#[derive(Debug, Clone)]
pub struct TraceEntry {
    /// Trace file identifier (filename stem).
    pub id: String,
    /// Timestamp string from the root span's `start_time`.
    pub timestamp: String,
    /// Name of the root span (first span in iteration order whose `parent_id` is `None`).
    /// When multiple roots exist (e.g., orphaned spans), only the first is used as the display name.
    pub root_span_name: String,
    /// Total number of spans in the trace.
    pub span_count: usize,
    /// Duration of the root span in milliseconds, if available.
    pub duration_ms: Option<f64>,
}

/// One flattened line of a span tree, ready for rendering.
#[derive(Debug, Clone)]
pub struct SpanLine {
    /// Indentation depth (0 = root).
    pub depth: usize,
    /// Span name.
    pub name: String,
    /// Duration in milliseconds, if available.
    pub duration_ms: Option<f64>,
}

/// Load trace files from `.assay/traces/`, sorted by mtime descending, capped at 20.
///
/// Skips unreadable or unparseable files with a `tracing::warn!`. Returns an empty
/// `Vec` if the traces directory does not exist.
pub fn load_traces(assay_dir: &Path) -> Vec<TraceEntry> {
    let traces_dir = assay_dir.join("traces");
    if !traces_dir.is_dir() {
        return Vec::new();
    }

    let entries = match std::fs::read_dir(&traces_dir) {
        Ok(rd) => rd,
        Err(e) => {
            tracing::warn!(
                dir = %traces_dir.display(),
                error = %e,
                "failed to read traces directory"
            );
            return Vec::new();
        }
    };

    // Collect (path, mtime) pairs for JSON files.
    let mut files: Vec<(std::path::PathBuf, std::time::SystemTime)> = entries
        .filter_map(|r| match r {
            Ok(entry) => Some(entry),
            Err(e) => {
                tracing::warn!(
                    dir = %traces_dir.display(),
                    error = %e,
                    "failed to read directory entry in traces dir"
                );
                None
            }
        })
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                return None;
            }
            let mtime = match entry.metadata() {
                Ok(m) => m.modified().unwrap_or_else(|e| {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "mtime unavailable; file will sort as oldest"
                    );
                    std::time::SystemTime::UNIX_EPOCH
                }),
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "metadata unavailable; skipping trace file"
                    );
                    return None;
                }
            };
            Some((path, mtime))
        })
        .collect();

    // Sort by mtime descending (most recent first).
    files.sort_by(|a, b| b.1.cmp(&a.1));

    // Cap at 20.
    files.truncate(20);

    let mut result = Vec::with_capacity(files.len());
    for (path, _mtime) in &files {
        let raw = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "skipping unreadable trace file"
                );
                continue;
            }
        };
        let spans: Vec<SpanData> = match serde_json::from_str(&raw) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "skipping unparseable trace file"
                );
                continue;
            }
        };

        let id = match path.file_stem() {
            Some(s) if !s.is_empty() => s.to_string_lossy().to_string(),
            _ => {
                tracing::warn!(path = %path.display(), "skipping trace file with no stem");
                continue;
            }
        };

        let root_span = spans.iter().find(|s| s.parent_id.is_none());
        let timestamp = root_span
            .map(|s| s.start_time.clone())
            .unwrap_or_else(|| "(unknown)".to_string());
        let root_span_name = root_span
            .map(|s| s.name.clone())
            .unwrap_or_else(|| "(unknown)".to_string());
        let duration_ms = root_span.and_then(|s| s.duration_ms);

        result.push(TraceEntry {
            id,
            timestamp,
            root_span_name,
            span_count: spans.len(),
            duration_ms,
        });
    }

    result
}

/// Load all spans from a single trace file by its ID.
///
/// Reads `<assay_dir>/traces/<trace_id>.json` and parses it as a `Vec<SpanData>`.
/// Returns an empty `Vec` on any error (with `tracing::warn!`).
pub fn load_trace_spans(assay_dir: &Path, trace_id: &str) -> Vec<SpanData> {
    let path = assay_dir.join("traces").join(format!("{trace_id}.json"));
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to read trace file for span expansion"
            );
            return Vec::new();
        }
    };
    match serde_json::from_str(&raw) {
        Ok(spans) => spans,
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to parse trace file for span expansion"
            );
            Vec::new()
        }
    }
}

/// Flatten a span tree into [`SpanLine`] entries for rendering.
///
/// Builds an adjacency map from `parent_id` → children. Orphan spans (whose
/// `parent_id` references a `span_id` not present in `spans`) are treated as
/// additional roots alongside spans with `parent_id: None`.
pub fn flatten_span_tree(spans: &[SpanData]) -> Vec<SpanLine> {
    if spans.is_empty() {
        return Vec::new();
    }

    // Build set of known span IDs.
    let known_ids: std::collections::HashSet<u64> = spans.iter().map(|s| s.span_id).collect();

    // Build adjacency map: parent_id → children.
    let mut children: HashMap<Option<u64>, Vec<&SpanData>> = HashMap::new();
    for span in spans {
        let effective_parent = span.parent_id.filter(|pid| known_ids.contains(pid));
        children.entry(effective_parent).or_default().push(span);
    }

    // Sort each child list by start_time.
    for list in children.values_mut() {
        list.sort_by(|a, b| a.start_time.cmp(&b.start_time));
    }

    // Recursively flatten from roots.
    let mut result = Vec::new();
    if let Some(roots) = children.get(&None) {
        for root in roots {
            flatten_recursive(root, &children, 0, &mut result);
        }
    }

    result
}

fn flatten_recursive(
    span: &SpanData,
    children: &HashMap<Option<u64>, Vec<&SpanData>>,
    depth: usize,
    out: &mut Vec<SpanLine>,
) {
    out.push(SpanLine {
        depth,
        name: span.name.clone(),
        duration_ms: span.duration_ms,
    });
    if let Some(kids) = children.get(&Some(span.span_id)) {
        for kid in kids {
            flatten_recursive(kid, children, depth + 1, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_core::telemetry::SpanData;
    use tempfile::TempDir;

    fn make_span(
        name: &str,
        span_id: u64,
        parent_id: Option<u64>,
        start_time: &str,
        duration_ms: Option<f64>,
    ) -> SpanData {
        SpanData {
            name: name.to_string(),
            target: "test".to_string(),
            level: "INFO".to_string(),
            span_id,
            parent_id,
            start_time: start_time.to_string(),
            end_time: None,
            duration_ms,
            fields: HashMap::new(),
        }
    }

    fn write_trace_file(dir: &Path, id: &str, spans: &[SpanData]) {
        let path = dir.join(format!("{id}.json"));
        let json = serde_json::to_string_pretty(spans).unwrap();
        std::fs::write(&path, json).unwrap();
    }

    // ── load_traces tests ────────────────────────────────────────

    #[test]
    fn test_load_traces_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let assay_dir = tmp.path().join(".assay");
        std::fs::create_dir_all(assay_dir.join("traces")).unwrap();
        let entries = load_traces(&assay_dir);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_load_traces_missing_dir() {
        let tmp = TempDir::new().unwrap();
        let assay_dir = tmp.path().join(".assay");
        // Don't create the traces dir at all.
        let entries = load_traces(&assay_dir);
        assert!(entries.is_empty());
    }

    #[test]
    fn test_load_traces_sorts_by_mtime() {
        let tmp = TempDir::new().unwrap();
        let traces_dir = tmp.path().join(".assay").join("traces");
        std::fs::create_dir_all(&traces_dir).unwrap();

        let spans_old = vec![make_span(
            "old-root",
            1,
            None,
            "2024-01-01T10:00:00Z",
            Some(10.0),
        )];
        let spans_new = vec![make_span(
            "new-root",
            2,
            None,
            "2024-01-02T10:00:00Z",
            Some(20.0),
        )];

        write_trace_file(&traces_dir, "old-trace", &spans_old);

        // Brief sleep to ensure different mtime.
        std::thread::sleep(std::time::Duration::from_millis(50));
        write_trace_file(&traces_dir, "new-trace", &spans_new);

        let assay_dir = tmp.path().join(".assay");
        let entries = load_traces(&assay_dir);
        assert_eq!(entries.len(), 2);
        // Most recent first.
        assert_eq!(entries[0].id, "new-trace");
        assert_eq!(entries[1].id, "old-trace");
    }

    #[test]
    fn test_load_traces_caps_at_20() {
        let tmp = TempDir::new().unwrap();
        let traces_dir = tmp.path().join(".assay").join("traces");
        std::fs::create_dir_all(&traces_dir).unwrap();

        for i in 0..25 {
            let spans = vec![make_span(
                &format!("root-{i}"),
                i as u64 + 1,
                None,
                &format!("2024-01-{:02}T10:00:00Z", (i % 28) + 1),
                Some(10.0),
            )];
            write_trace_file(&traces_dir, &format!("trace-{i:03}"), &spans);
        }

        let assay_dir = tmp.path().join(".assay");
        let entries = load_traces(&assay_dir);
        assert_eq!(entries.len(), 20);
    }

    #[test]
    fn test_load_traces_skips_bad_files() {
        let tmp = TempDir::new().unwrap();
        let traces_dir = tmp.path().join(".assay").join("traces");
        std::fs::create_dir_all(&traces_dir).unwrap();

        // One valid trace.
        let spans = vec![make_span(
            "good",
            1,
            None,
            "2024-01-01T10:00:00Z",
            Some(10.0),
        )];
        write_trace_file(&traces_dir, "good-trace", &spans);

        // One invalid JSON file.
        std::fs::write(traces_dir.join("bad-trace.json"), "not valid json").unwrap();

        let assay_dir = tmp.path().join(".assay");
        let entries = load_traces(&assay_dir);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "good-trace");
    }

    // ── flatten_span_tree tests ──────────────────────────────────

    #[test]
    fn test_flatten_span_tree_basic() {
        let spans = vec![
            make_span("root", 1, None, "2024-01-01T12:00:00Z", Some(100.0)),
            make_span("child-a", 2, Some(1), "2024-01-01T12:00:01Z", Some(30.0)),
            make_span("child-b", 3, Some(1), "2024-01-01T12:00:02Z", Some(40.0)),
            make_span("grandchild", 4, Some(2), "2024-01-01T12:00:01Z", Some(20.0)),
        ];

        let lines = flatten_span_tree(&spans);
        assert_eq!(lines.len(), 4);

        assert_eq!(lines[0].name, "root");
        assert_eq!(lines[0].depth, 0);

        assert_eq!(lines[1].name, "child-a");
        assert_eq!(lines[1].depth, 1);

        assert_eq!(lines[2].name, "grandchild");
        assert_eq!(lines[2].depth, 2);

        assert_eq!(lines[3].name, "child-b");
        assert_eq!(lines[3].depth, 1);
    }

    #[test]
    fn test_flatten_span_tree_orphan_roots() {
        let spans = vec![
            make_span("root", 1, None, "2024-01-01T12:00:00Z", Some(100.0)),
            make_span("child", 2, Some(1), "2024-01-01T12:00:01Z", Some(30.0)),
            // Orphan: parent_id 999 doesn't exist in the span set.
            make_span("orphan", 3, Some(999), "2024-01-01T12:00:02Z", Some(10.0)),
        ];

        let lines = flatten_span_tree(&spans);
        assert_eq!(lines.len(), 3);

        // Root and orphan should both appear at depth 0.
        assert_eq!(lines[0].name, "root");
        assert_eq!(lines[0].depth, 0);

        assert_eq!(lines[1].name, "child");
        assert_eq!(lines[1].depth, 1);

        assert_eq!(lines[2].name, "orphan");
        assert_eq!(lines[2].depth, 0);
    }

    #[test]
    fn test_flatten_span_tree_empty() {
        let lines = flatten_span_tree(&[]);
        assert!(lines.is_empty());
    }
}
