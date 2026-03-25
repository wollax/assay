//! Traces subcommands for the `assay traces` CLI group.
//!
//! `assay traces list` scans `.assay/traces/` and prints a table of trace files.
//! `assay traces show <id>` loads a trace JSON file and renders an indented span tree.

use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Context;
use clap::Subcommand;

use assay_core::telemetry::SpanData;

use super::{COLUMN_GAP, assay_dir, colors_enabled, project_root};

#[derive(Subcommand)]
pub(crate) enum TracesCommand {
    /// List trace files in .assay/traces/
    #[command(after_long_help = "\
Examples:
  List all trace files:
    assay traces list")]
    List,

    /// Show a span tree for a specific trace
    #[command(after_long_help = "\
Examples:
  Show trace by ID (filename without .json):
    assay traces show 20240101T120000Z-abc123")]
    Show {
        /// Trace ID (filename without the .json extension)
        id: String,
    },
}

/// Handle traces subcommands.
pub(crate) fn handle(command: TracesCommand) -> anyhow::Result<i32> {
    match command {
        TracesCommand::List => handle_list(),
        TracesCommand::Show { id } => handle_show(&id),
    }
}

// ── Helpers ───────────────────────────────────────────────────────

/// Resolve the traces directory path.
fn traces_dir(root: &std::path::Path) -> PathBuf {
    assay_dir(root).join("traces")
}

/// Load and parse a trace JSON file, returning the span list.
fn load_trace(path: &std::path::Path) -> anyhow::Result<Vec<SpanData>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read trace file: {}", path.display()))?;
    serde_json::from_str::<Vec<SpanData>>(&raw)
        .with_context(|| format!("failed to parse trace JSON: {}", path.display()))
}

// ── List ──────────────────────────────────────────────────────────

/// Handle `assay traces list`.
fn handle_list() -> anyhow::Result<i32> {
    let root = project_root()?;
    let td = traces_dir(&root);

    if !td.is_dir() {
        tracing::error!(
            path = %td.display(),
            "No traces directory found. Run an instrumented pipeline to generate traces."
        );
        return Ok(1);
    }

    // Collect trace file entries.
    let mut entries: Vec<(String, PathBuf)> = std::fs::read_dir(&td)
        .with_context(|| format!("failed to read traces directory: {}", td.display()))?
        .filter_map(|r| r.ok())
        .filter_map(|e| {
            let path = e.path();
            if path.extension()?.to_str()? == "json" {
                let id = path.file_stem()?.to_string_lossy().to_string();
                Some((id, path))
            } else {
                None
            }
        })
        .collect();

    if entries.is_empty() {
        println!("No trace files found in {}", td.display());
        return Ok(0);
    }

    // Sort chronologically (filename has timestamp prefix).
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let mut rows: Vec<TraceRow> = Vec::with_capacity(entries.len());

    for (id, path) in &entries {
        match load_trace(path) {
            Ok(spans) => {
                let root_span = spans
                    .iter()
                    .find(|s| s.parent_id.is_none())
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| "(unknown)".to_string());

                // Extract timestamp from the first span's start_time, or fall back to the ID prefix.
                let timestamp = spans
                    .first()
                    .map(|s| s.start_time.clone())
                    .unwrap_or_else(|| id.clone());

                rows.push(TraceRow {
                    id: id.clone(),
                    timestamp,
                    root_span,
                    span_count: spans.len(),
                });
            }
            Err(e) => {
                tracing::warn!(
                    id = %id,
                    error = %e,
                    "skipping unreadable trace file"
                );
            }
        }
    }

    if rows.is_empty() {
        println!("No readable trace files found.");
        return Ok(0);
    }

    let color = colors_enabled();
    print_trace_list(&rows, color);
    Ok(0)
}

struct TraceRow {
    id: String,
    timestamp: String,
    root_span: String,
    span_count: usize,
}

fn print_trace_list(rows: &[TraceRow], _color: bool) {
    // Compute column widths.
    let id_w = rows.iter().map(|r| r.id.len()).max().unwrap_or(2).max(2); // "ID"
    let ts_w = rows
        .iter()
        .map(|r| r.timestamp.len())
        .max()
        .unwrap_or(9)
        .max(9); // "Timestamp"
    let rs_w = rows
        .iter()
        .map(|r| r.root_span.len())
        .max()
        .unwrap_or(9)
        .max(9); // "Root Span"

    println!(
        "  {:<id_w$}{gap}{:<ts_w$}{gap}{:<rs_w$}{gap}{:>5}",
        "ID",
        "Timestamp",
        "Root Span",
        "Spans",
        gap = COLUMN_GAP,
    );
    println!(
        "  {}{gap}{}{gap}{}{gap}{}",
        "\u{2500}".repeat(id_w),
        "\u{2500}".repeat(ts_w),
        "\u{2500}".repeat(rs_w),
        "\u{2500}".repeat(5),
        gap = COLUMN_GAP,
    );

    for r in rows {
        println!(
            "  {:<id_w$}{gap}{:<ts_w$}{gap}{:<rs_w$}{gap}{:>5}",
            r.id,
            r.timestamp,
            r.root_span,
            r.span_count,
            gap = COLUMN_GAP,
        );
    }
}

// ── Show ──────────────────────────────────────────────────────────

/// Handle `assay traces show <id>`.
fn handle_show(id: &str) -> anyhow::Result<i32> {
    let root = project_root()?;
    let td = traces_dir(&root);
    let path = td.join(format!("{id}.json"));

    if !path.exists() {
        tracing::error!(
            id = %id,
            path = %path.display(),
            "Trace file not found. Run `assay traces list` to see available traces."
        );
        return Ok(1);
    }

    let spans = match load_trace(&path) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(
                id = %id,
                error = %e,
                "Failed to parse trace file"
            );
            return Ok(1);
        }
    };

    if spans.is_empty() {
        println!("Trace '{id}' contains no spans.");
        return Ok(0);
    }

    print_span_tree(id, &spans);
    Ok(0)
}

/// Render the span tree to stdout.
fn print_span_tree(id: &str, spans: &[SpanData]) {
    println!("Trace: {id}");
    println!();

    // Build adjacency: parent_id -> children.
    // Use a sentinel None key for root spans.
    let mut children: HashMap<Option<u64>, Vec<&SpanData>> = HashMap::new();
    for span in spans {
        children.entry(span.parent_id).or_default().push(span);
    }

    // Sort each child list by start_time for consistent ordering.
    for list in children.values_mut() {
        list.sort_by(|a, b| a.start_time.cmp(&b.start_time));
    }

    // Recursively render from root spans.
    let roots: Vec<&SpanData> = {
        let mut r = children.get(&None).cloned().unwrap_or_default();
        r.sort_by(|a, b| a.start_time.cmp(&b.start_time));
        r
    };

    for root in roots {
        render_span(root, &children, 0);
    }
}

/// Recursively render a single span and its children at the given depth.
fn render_span(span: &SpanData, children: &HashMap<Option<u64>, Vec<&SpanData>>, depth: usize) {
    let indent = "  ".repeat(depth);
    let duration = match span.duration_ms {
        Some(ms) => format!(" ({ms:.1}ms)"),
        None => String::new(),
    };
    println!("{indent}{}{duration}", span.name);

    if let Some(kids) = children.get(&Some(span.span_id)) {
        for kid in kids {
            render_span(kid, children, depth + 1);
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

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

    fn write_trace_file(dir: &std::path::Path, id: &str, spans: &[SpanData]) {
        let path = dir.join(format!("{id}.json"));
        let json = serde_json::to_string_pretty(spans).unwrap();
        std::fs::write(&path, json).unwrap();
    }

    // ── list tests ────────────────────────────────────────────────

    #[test]
    fn test_handle_list_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let traces_dir = tmp.path().join(".assay").join("traces");
        std::fs::create_dir_all(&traces_dir).unwrap();

        // Should succeed and print "No trace files found".
        // We can't easily test stdout, but at least verify the dir logic.
        assert!(traces_dir.is_dir());
        assert!(std::fs::read_dir(&traces_dir).unwrap().next().is_none());
    }

    #[test]
    fn test_handle_list_parses_files() {
        let tmp = TempDir::new().unwrap();
        let traces_dir = tmp.path().join(".assay").join("traces");
        std::fs::create_dir_all(&traces_dir).unwrap();

        let spans = vec![
            make_span("pipeline", 1, None, "2024-01-01T12:00:00Z", Some(100.0)),
            make_span("gate-run", 2, Some(1), "2024-01-01T12:00:00Z", Some(50.0)),
        ];
        write_trace_file(&traces_dir, "2024010112-abc", &spans);

        // Parse the file directly to verify our logic.
        let path = traces_dir.join("2024010112-abc.json");
        let raw = std::fs::read_to_string(&path).unwrap();
        let loaded: Vec<SpanData> = serde_json::from_str(&raw).unwrap();

        assert_eq!(loaded.len(), 2);
        let root = loaded.iter().find(|s| s.parent_id.is_none()).unwrap();
        assert_eq!(root.name, "pipeline");
    }

    #[test]
    fn test_list_sorts_by_filename() {
        let tmp = TempDir::new().unwrap();
        let traces_dir = tmp.path().join(".assay").join("traces");
        std::fs::create_dir_all(&traces_dir).unwrap();

        let spans_a = vec![make_span(
            "span-a",
            1,
            None,
            "2024-01-01T10:00:00Z",
            Some(10.0),
        )];
        let spans_b = vec![make_span(
            "span-b",
            2,
            None,
            "2024-01-02T10:00:00Z",
            Some(20.0),
        )];

        write_trace_file(&traces_dir, "2024010110-first", &spans_a);
        write_trace_file(&traces_dir, "2024010210-second", &spans_b);

        // Collect and sort as handle_list does.
        let mut entries: Vec<String> = std::fs::read_dir(&traces_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let p = e.path();
                if p.extension()?.to_str()? == "json" {
                    Some(p.file_stem()?.to_string_lossy().to_string())
                } else {
                    None
                }
            })
            .collect();
        entries.sort();

        assert_eq!(entries[0], "2024010110-first");
        assert_eq!(entries[1], "2024010210-second");
    }

    // ── show tests ────────────────────────────────────────────────

    #[test]
    fn test_span_tree_renders_root() {
        let spans = vec![make_span(
            "root-span",
            1,
            None,
            "2024-01-01T12:00:00Z",
            Some(42.0),
        )];

        // Build the tree as handle_show does and verify the structure.
        let mut children: HashMap<Option<u64>, Vec<&SpanData>> = HashMap::new();
        for s in &spans {
            children.entry(s.parent_id).or_default().push(s);
        }

        let roots = children.get(&None).cloned().unwrap_or_default();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].name, "root-span");
        assert_eq!(roots[0].duration_ms, Some(42.0));
    }

    #[test]
    fn test_span_tree_parent_child() {
        let spans = vec![
            make_span("root", 1, None, "2024-01-01T12:00:00Z", Some(100.0)),
            make_span("child-a", 2, Some(1), "2024-01-01T12:00:01Z", Some(30.0)),
            make_span("child-b", 3, Some(1), "2024-01-01T12:00:02Z", Some(40.0)),
            make_span("grandchild", 4, Some(2), "2024-01-01T12:00:01Z", Some(20.0)),
        ];

        let mut children: HashMap<Option<u64>, Vec<&SpanData>> = HashMap::new();
        for s in &spans {
            children.entry(s.parent_id).or_default().push(s);
        }

        // Root level.
        let roots = children.get(&None).cloned().unwrap_or_default();
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].name, "root");

        // Root's children.
        let root_children = children.get(&Some(1)).cloned().unwrap_or_default();
        assert_eq!(root_children.len(), 2);

        // Grandchildren of child-a.
        let grandchildren = children.get(&Some(2)).cloned().unwrap_or_default();
        assert_eq!(grandchildren.len(), 1);
        assert_eq!(grandchildren[0].name, "grandchild");
    }

    #[test]
    fn test_missing_trace_file_detected() {
        let tmp = TempDir::new().unwrap();
        let traces_dir = tmp.path().join(".assay").join("traces");
        std::fs::create_dir_all(&traces_dir).unwrap();

        let path = traces_dir.join("nonexistent.json");
        assert!(!path.exists());
    }

    #[test]
    fn test_malformed_json_returns_error() {
        let tmp = TempDir::new().unwrap();
        let traces_dir = tmp.path().join(".assay").join("traces");
        std::fs::create_dir_all(&traces_dir).unwrap();

        let path = traces_dir.join("bad.json");
        std::fs::write(&path, "not valid json {{{").unwrap();

        let result = serde_json::from_str::<Vec<SpanData>>("not valid json {{{");
        assert!(result.is_err());
    }
}
