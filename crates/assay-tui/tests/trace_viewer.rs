//! Integration tests for the Trace Viewer screen.
//!
//! Run with:
//!   cargo test -p assay-tui --test trace_viewer

use std::collections::HashMap;
use std::path::PathBuf;

use assay_core::telemetry::SpanData;
use assay_tui::app::{App, Screen};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a key press event with no modifiers.
fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

/// Create a minimal fixture project with `.assay/` directory and optional traces dir.
/// Returns the project root path.
fn setup_project(tmp: &TempDir) -> PathBuf {
    let root = tmp.path().to_path_buf();
    let assay_dir = root.join(".assay");

    std::fs::create_dir_all(assay_dir.join("milestones")).unwrap();
    std::fs::create_dir_all(assay_dir.join("traces")).unwrap();

    // Minimal milestone so scan succeeds.
    std::fs::write(
        assay_dir.join("milestones").join("alpha.toml"),
        r#"slug = "alpha"
name = "Alpha"
status = "draft"
created_at = "2026-01-01T00:00:00Z"
updated_at = "2026-01-01T00:00:00Z"

[[chunks]]
slug = "c1"
order = 1
"#,
    )
    .unwrap();

    root
}

/// Create a minimal fixture project without a traces directory.
fn setup_project_no_traces(tmp: &TempDir) -> PathBuf {
    let root = tmp.path().to_path_buf();
    let assay_dir = root.join(".assay");

    std::fs::create_dir_all(assay_dir.join("milestones")).unwrap();

    std::fs::write(
        assay_dir.join("milestones").join("alpha.toml"),
        r#"slug = "alpha"
name = "Alpha"
status = "draft"
created_at = "2026-01-01T00:00:00Z"
updated_at = "2026-01-01T00:00:00Z"

[[chunks]]
slug = "c1"
order = 1
"#,
    )
    .unwrap();

    root
}

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

// ── Screen transition tests ───────────────────────────────────────────────────

#[test]
fn test_t_key_transitions_to_trace_viewer() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);

    // Write a trace file so we have data.
    let traces_dir = root.join(".assay").join("traces");
    let spans = vec![
        make_span("pipeline", 1, None, "2024-01-01T12:00:00Z", Some(100.0)),
        make_span("gate-run", 2, Some(1), "2024-01-01T12:00:00Z", Some(50.0)),
    ];
    write_trace_file(&traces_dir, "test-trace", &spans);

    let mut app = App::with_project_root(Some(root)).unwrap();
    assert!(matches!(app.screen, Screen::Dashboard));

    app.handle_event(key(KeyCode::Char('t')));
    assert!(
        matches!(app.screen, Screen::TraceViewer { .. }),
        "expected Screen::TraceViewer after pressing 't', got {:?}",
        screen_name(&app.screen)
    );
}

#[test]
fn test_esc_from_trace_list_returns_to_dashboard() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    app.handle_event(key(KeyCode::Char('t')));
    assert!(matches!(app.screen, Screen::TraceViewer { .. }));

    app.handle_event(key(KeyCode::Esc));
    assert!(
        matches!(app.screen, Screen::Dashboard),
        "expected Screen::Dashboard after Esc from TraceViewer"
    );
}

#[test]
fn test_empty_traces_dir_shows_trace_viewer() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project_no_traces(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    app.handle_event(key(KeyCode::Char('t')));
    match &app.screen {
        Screen::TraceViewer { traces, .. } => {
            assert!(
                traces.is_empty(),
                "expected empty traces vec when no traces dir exists"
            );
        }
        _ => panic!(
            "expected Screen::TraceViewer, got {:?}",
            screen_name(&app.screen)
        ),
    }
}

/// Helper to get a debug-friendly screen name.
fn screen_name(screen: &Screen) -> &'static str {
    match screen {
        Screen::NoProject => "NoProject",
        Screen::Dashboard => "Dashboard",
        Screen::Wizard(_) => "Wizard",
        Screen::LoadError(_) => "LoadError",
        Screen::MilestoneDetail { .. } => "MilestoneDetail",
        Screen::ChunkDetail { .. } => "ChunkDetail",
        Screen::Settings { .. } => "Settings",
        Screen::AgentRun { .. } => "AgentRun",
        Screen::Analytics => "Analytics",
        Screen::McpPanel { .. } => "McpPanel",
        Screen::TraceViewer { .. } => "TraceViewer",
    }
}
