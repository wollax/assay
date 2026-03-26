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

/// Collect all JSON trace files from `dir`, return sorted by filename.
fn read_all_traces(dir: &Path) -> Vec<(String, Vec<SpanData>)> {
    let mut files: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "json"))
        .map(|e| e.path())
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let id = p.file_stem().unwrap().to_string_lossy().into_owned();
            let contents = std::fs::read_to_string(&p).unwrap();
            let spans: Vec<SpanData> = serde_json::from_str(&contents).unwrap();
            (id, spans)
        })
        .collect()
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

/// End-to-end round-trip: write spans via JsonFileLayer, read back JSON file,
/// verify tree structure matches expectations (write → read → render cycle).
///
/// This is the T03 integration test that proves the full loop:
/// 1. JsonFileLayer writes JSON trace file with correct structure
/// 2. SpanData can be deserialized back from JSON
/// 3. Parent-child relationships are preserved for tree rendering
/// 4. Trace ID (filename stem) is stable and readable
#[test]
fn trace_export_end_to_end_write_read_render() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();

    // Phase 1: Write — instrument a 3-level span tree via real subscriber.
    with_json_layer(&dir, 50, || {
        let root = tracing::info_span!("orchestration_run", run_id = "test-run-001");
        let _root_guard = root.enter();

        {
            let session = tracing::info_span!("session", session_name = "auth-spec");
            let _session_guard = session.enter();

            {
                let stage = tracing::info_span!("gate_eval", gate = "unit-tests");
                let _stage_guard = stage.enter();
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
        }

        {
            let merge = tracing::info_span!("merge_propose", target_branch = "main");
            let _merge_guard = merge.enter();
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    });

    // Phase 2: Read — verify exactly one JSON file was written.
    let traces = read_all_traces(&dir);
    assert_eq!(
        traces.len(),
        1,
        "expected exactly 1 trace file after one root span"
    );

    let (trace_id, spans) = &traces[0];
    // Trace ID must be non-empty and look like a timestamp-hex filename.
    assert!(!trace_id.is_empty(), "trace ID must be non-empty");
    assert!(
        trace_id.contains('T') || trace_id.len() > 8,
        "trace ID should be timestamp-based, got: {trace_id}"
    );

    // Phase 3: Verify tree structure — correct spans, parent-child, timing.
    assert_eq!(
        spans.len(),
        4,
        "expected 4 spans (root, session, gate_eval, merge_propose), got {}: {:?}",
        spans.len(),
        spans.iter().map(|s| &s.name).collect::<Vec<_>>()
    );

    // Root span checks.
    let root = spans
        .iter()
        .find(|s| s.name == "orchestration_run")
        .expect("root span missing");
    assert!(root.parent_id.is_none(), "root must have no parent_id");
    assert!(
        root.duration_ms.unwrap_or(0.0) > 0.0,
        "root duration must be > 0"
    );
    assert_eq!(
        root.fields.get("run_id").and_then(|v| v.as_str()),
        Some("test-run-001"),
        "root span field 'run_id' must be captured"
    );

    // Session child.
    let session = spans
        .iter()
        .find(|s| s.name == "session")
        .expect("session span missing");
    assert_eq!(
        session.parent_id,
        Some(root.span_id),
        "session must be child of root"
    );
    assert_eq!(
        session.fields.get("session_name").and_then(|v| v.as_str()),
        Some("auth-spec")
    );

    // Gate eval grandchild (child of session).
    let gate_eval = spans
        .iter()
        .find(|s| s.name == "gate_eval")
        .expect("gate_eval span missing");
    assert_eq!(
        gate_eval.parent_id,
        Some(session.span_id),
        "gate_eval must be child of session"
    );
    assert_eq!(
        gate_eval.fields.get("gate").and_then(|v| v.as_str()),
        Some("unit-tests")
    );
    assert!(
        gate_eval.duration_ms.unwrap_or(0.0) > 0.0,
        "gate_eval must have positive duration"
    );

    // Merge propose child (child of root, sibling of session).
    let merge = spans
        .iter()
        .find(|s| s.name == "merge_propose")
        .expect("merge_propose missing");
    assert_eq!(
        merge.parent_id,
        Some(root.span_id),
        "merge_propose must be child of root"
    );
    assert_eq!(
        merge.fields.get("target_branch").and_then(|v| v.as_str()),
        Some("main")
    );

    // Phase 4: Render simulation — verify tree can be reconstructed for CLI show.
    // Build adjacency map (same logic as assay-cli traces show).
    let mut children: std::collections::HashMap<Option<u64>, Vec<&SpanData>> =
        std::collections::HashMap::new();
    for span in spans {
        children.entry(span.parent_id).or_default().push(span);
    }
    // Root level should have: orchestration_run
    let roots = children.get(&None).expect("must have root-level spans");
    assert_eq!(roots.len(), 1, "exactly one root span for tree rendering");
    assert_eq!(roots[0].name, "orchestration_run");

    // Children of root: session + merge_propose
    let root_children = children
        .get(&Some(root.span_id))
        .expect("root must have children");
    assert_eq!(
        root_children.len(),
        2,
        "root must have 2 children: session + merge_propose"
    );
    let child_names: std::collections::HashSet<&str> =
        root_children.iter().map(|s| s.name.as_str()).collect();
    assert!(child_names.contains("session"));
    assert!(child_names.contains("merge_propose"));

    // gate_eval is the only grandchild.
    let session_children = children
        .get(&Some(session.span_id))
        .expect("session must have children");
    assert_eq!(session_children.len(), 1);
    assert_eq!(session_children[0].name, "gate_eval");
}

/// Verify that `on_record` merges fields added after span creation without
/// overwriting fields that were set at creation time.
///
/// `span.record()` only works for fields declared in the span macro (either
/// with a value or with `tracing::field::Empty` as a placeholder). Fields
/// not declared in the macro are silently ignored by tracing.
#[test]
fn trace_export_on_record_merges_fields() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path().to_path_buf();

    with_json_layer(&dir, 50, || {
        // Declare all fields upfront; use Empty as a placeholder for fields
        // that will be recorded later via span.record().
        let root = tracing::info_span!(
            "pipeline_run",
            initial_field = "set_at_creation",
            added_later = tracing::field::Empty,
            numeric_later = tracing::field::Empty,
        );
        let _guard = root.enter();
        // Record the placeholder fields after span creation.
        root.record("added_later", "recorded_value");
        root.record("numeric_later", 42_i64);
    });

    let spans = read_single_trace(&dir);
    let root = spans
        .iter()
        .find(|s| s.name == "pipeline_run")
        .expect("root span missing");

    // Field set at creation must survive.
    assert_eq!(
        root.fields.get("initial_field").and_then(|v| v.as_str()),
        Some("set_at_creation"),
        "on_record must not overwrite fields set at creation"
    );
    // Fields added via span.record() must be captured.
    assert_eq!(
        root.fields.get("added_later").and_then(|v| v.as_str()),
        Some("recorded_value"),
        "on_record must capture string fields added after creation"
    );
    assert_eq!(
        root.fields.get("numeric_later").and_then(|v| v.as_i64()),
        Some(42),
        "on_record must capture integer fields added after creation"
    );
}
