//! Integration tests for the `JsonFileLayer` trace export.
//!
//! Each test creates its own subscriber (not global) via `set_default` to
//! avoid conflicts with the process-global subscriber.

use std::path::Path;

use assay_core::telemetry::{JsonFileLayer, SpanData};
use tempfile::TempDir;
use tracing_subscriber::layer::SubscriberExt;

/// Helper: build a subscriber with a `JsonFileLayer` targeting `dir` and
/// execute `f` within it using `tracing::subscriber::with_default`.
fn with_json_layer<F: FnOnce()>(dir: &Path, max_files: usize, f: F) {
    let layer = JsonFileLayer::new(dir.to_path_buf(), max_files);
    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::with_default(subscriber, f);
}

/// Read the single JSON trace file from `dir`, panic if zero or >1 files.
fn read_single_trace(dir: &Path) -> Vec<SpanData> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .map(|e| e.path())
        .collect();
    assert_eq!(
        files.len(),
        1,
        "expected exactly 1 trace file, found {}",
        files.len()
    );
    let contents = std::fs::read_to_string(files.remove(0)).unwrap();
    serde_json::from_str(&contents).unwrap()
}

/// Count JSON files in the directory.
fn json_file_count(dir: &Path) -> usize {
    std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .count()
}

#[test]
fn trace_export_creates_json_file_with_correct_tree() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();

    with_json_layer(&dir, 50, || {
        let root = tracing::info_span!("pipeline_run", pipeline = "test-pipe");
        let _root_guard = root.enter();

        {
            let child = tracing::info_span!("stage_a", stage_index = 1);
            let _child_guard = child.enter();

            {
                let grandchild = tracing::debug_span!("gate_check", gate = "lint");
                let _gc_guard = grandchild.enter();
                // simulate some work
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }

        {
            let child2 = tracing::info_span!("stage_b", stage_index = 2);
            let _child2_guard = child2.enter();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    });

    let spans = read_single_trace(&dir);

    // Should have 4 spans: pipeline_run, stage_a, gate_check, stage_b
    assert_eq!(
        spans.len(),
        4,
        "expected 4 spans, got {}: {:?}",
        spans.len(),
        spans.iter().map(|s| &s.name).collect::<Vec<_>>()
    );

    // Find the root span.
    let root = spans.iter().find(|s| s.name == "pipeline_run").unwrap();
    assert!(root.parent_id.is_none(), "root should have no parent");
    assert!(
        root.duration_ms.unwrap() > 0.0,
        "root duration should be > 0"
    );
    assert!(root.end_time.is_some(), "root should have end_time");
    // Check field was captured
    assert_eq!(
        root.fields.get("pipeline").and_then(|v| v.as_str()),
        Some("test-pipe")
    );

    // Find child spans and verify parent relationships.
    let stage_a = spans.iter().find(|s| s.name == "stage_a").unwrap();
    assert_eq!(stage_a.parent_id, Some(root.span_id));
    assert_eq!(
        stage_a.fields.get("stage_index").and_then(|v| v.as_u64()),
        Some(1)
    );

    let gate_check = spans.iter().find(|s| s.name == "gate_check").unwrap();
    assert_eq!(gate_check.parent_id, Some(stage_a.span_id));
    assert_eq!(gate_check.level, "DEBUG");

    let stage_b = spans.iter().find(|s| s.name == "stage_b").unwrap();
    assert_eq!(stage_b.parent_id, Some(root.span_id));

    // All spans should have timing.
    for span in &spans {
        assert!(
            span.duration_ms.is_some(),
            "span {} missing duration",
            span.name
        );
        assert!(
            span.end_time.is_some(),
            "span {} missing end_time",
            span.name
        );
    }
}

#[test]
fn trace_export_prunes_old_files() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();

    // Create 55 dummy JSON files to exceed the max of 50.
    for i in 0..55 {
        let name = format!("20260101T{:06}Z-{:06x}.json", i, i);
        std::fs::write(dir.join(name), "[]").unwrap();
    }
    assert_eq!(json_file_count(&dir), 55);

    // Run a trace with max_files = 50 — should write 1 new file then prune.
    with_json_layer(&dir, 50, || {
        let root = tracing::info_span!("prune_test");
        let _guard = root.enter();
    });

    // After write + prune: should be at most 50 files.
    let count = json_file_count(&dir);
    assert!(
        count <= 50,
        "expected <= 50 files after pruning, got {count}"
    );
}

#[test]
fn trace_export_multiple_root_spans_produce_multiple_files() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();

    with_json_layer(&dir, 50, || {
        // First root span
        {
            let root1 = tracing::info_span!("trace_one");
            let _guard = root1.enter();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }

        // Small delay to ensure different trace IDs.
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Second root span
        {
            let root2 = tracing::info_span!("trace_two");
            let _guard = root2.enter();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });

    let count = json_file_count(&dir);
    assert_eq!(
        count, 2,
        "expected 2 trace files for 2 root spans, got {count}"
    );
}
