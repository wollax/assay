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
    match &app.screen {
        Screen::TraceViewer {
            traces,
            trace_list_state,
            selected_trace,
            ..
        } => {
            assert_eq!(traces.len(), 1);
            assert_eq!(trace_list_state.selected(), Some(0));
            assert!(selected_trace.is_none(), "should start in trace list mode");
        }
        _ => panic!(
            "expected Screen::TraceViewer after pressing 't', got {:?}",
            screen_name(&app.screen)
        ),
    }
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
        Screen::TraceViewer {
            traces,
            trace_list_state,
            ..
        } => {
            assert!(
                traces.is_empty(),
                "expected empty traces vec when no traces dir exists"
            );
            assert_eq!(
                trace_list_state.selected(),
                None,
                "selection should be None when traces is empty"
            );
        }
        _ => panic!(
            "expected Screen::TraceViewer, got {:?}",
            screen_name(&app.screen)
        ),
    }
}

#[test]
fn test_enter_expands_span_tree_and_esc_returns() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let traces_dir = root.join(".assay").join("traces");

    let spans = vec![
        make_span("pipeline", 1, None, "2024-01-01T12:00:00Z", Some(100.0)),
        make_span("gate-run", 2, Some(1), "2024-01-01T12:00:01Z", Some(50.0)),
    ];
    write_trace_file(&traces_dir, "test-trace", &spans);

    let mut app = App::with_project_root(Some(root)).unwrap();
    app.handle_event(key(KeyCode::Char('t')));

    // Press Enter to expand span tree.
    app.handle_event(key(KeyCode::Enter));
    match &app.screen {
        Screen::TraceViewer {
            selected_trace,
            span_lines,
            span_list_state,
            ..
        } => {
            assert_eq!(*selected_trace, Some(0), "should be in span tree mode");
            assert_eq!(span_lines.len(), 2, "should have 2 span lines");
            assert_eq!(span_lines[0].name, "pipeline");
            assert_eq!(span_lines[1].name, "gate-run");
            assert_eq!(span_list_state.selected(), Some(0));
        }
        _ => panic!("expected TraceViewer in span mode"),
    }

    // Press Esc to return to trace list.
    app.handle_event(key(KeyCode::Esc));
    match &app.screen {
        Screen::TraceViewer {
            selected_trace,
            span_lines,
            ..
        } => {
            assert!(
                selected_trace.is_none(),
                "should be back in trace list mode"
            );
            assert!(span_lines.is_empty(), "span_lines should be cleared");
        }
        _ => panic!("expected TraceViewer in list mode"),
    }

    // Press Esc again to return to Dashboard.
    app.handle_event(key(KeyCode::Esc));
    assert!(
        matches!(app.screen, Screen::Dashboard),
        "expected Dashboard after second Esc"
    );
}

#[test]
fn test_up_down_navigation_in_trace_list() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let traces_dir = root.join(".assay").join("traces");

    // Write two trace files with different mtimes.
    let spans1 = vec![make_span(
        "first",
        1,
        None,
        "2024-01-01T10:00:00Z",
        Some(10.0),
    )];
    write_trace_file(&traces_dir, "trace-a", &spans1);
    std::thread::sleep(std::time::Duration::from_millis(50));
    let spans2 = vec![make_span(
        "second",
        2,
        None,
        "2024-01-02T10:00:00Z",
        Some(20.0),
    )];
    write_trace_file(&traces_dir, "trace-b", &spans2);

    let mut app = App::with_project_root(Some(root)).unwrap();
    app.handle_event(key(KeyCode::Char('t')));

    // Initially selected at 0.
    if let Screen::TraceViewer {
        trace_list_state, ..
    } = &app.screen
    {
        assert_eq!(trace_list_state.selected(), Some(0));
    }

    // Down → 1.
    app.handle_event(key(KeyCode::Down));
    if let Screen::TraceViewer {
        trace_list_state, ..
    } = &app.screen
    {
        assert_eq!(trace_list_state.selected(), Some(1));
    }

    // Down again → still 1 (clamped).
    app.handle_event(key(KeyCode::Down));
    if let Screen::TraceViewer {
        trace_list_state, ..
    } = &app.screen
    {
        assert_eq!(trace_list_state.selected(), Some(1));
    }

    // Up → 0.
    app.handle_event(key(KeyCode::Up));
    if let Screen::TraceViewer {
        trace_list_state, ..
    } = &app.screen
    {
        assert_eq!(trace_list_state.selected(), Some(0));
    }

    // Up again → still 0 (clamped).
    app.handle_event(key(KeyCode::Up));
    if let Screen::TraceViewer {
        trace_list_state, ..
    } = &app.screen
    {
        assert_eq!(trace_list_state.selected(), Some(0));
    }
}

#[test]
fn test_up_down_navigation_in_span_tree() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let traces_dir = root.join(".assay").join("traces");

    let spans = vec![
        make_span("root", 1, None, "2024-01-01T12:00:00Z", Some(100.0)),
        make_span("child-a", 2, Some(1), "2024-01-01T12:00:01Z", Some(30.0)),
        make_span("child-b", 3, Some(1), "2024-01-01T12:00:02Z", Some(40.0)),
    ];
    write_trace_file(&traces_dir, "test-trace", &spans);

    let mut app = App::with_project_root(Some(root)).unwrap();
    app.handle_event(key(KeyCode::Char('t')));
    app.handle_event(key(KeyCode::Enter)); // expand span tree

    // Should start at 0.
    if let Screen::TraceViewer {
        span_list_state, ..
    } = &app.screen
    {
        assert_eq!(span_list_state.selected(), Some(0));
    }

    // Down → 1, Down → 2.
    app.handle_event(key(KeyCode::Down));
    if let Screen::TraceViewer {
        span_list_state, ..
    } = &app.screen
    {
        assert_eq!(span_list_state.selected(), Some(1));
    }
    app.handle_event(key(KeyCode::Down));
    if let Screen::TraceViewer {
        span_list_state, ..
    } = &app.screen
    {
        assert_eq!(span_list_state.selected(), Some(2));
    }

    // Down again → clamped at 2.
    app.handle_event(key(KeyCode::Down));
    if let Screen::TraceViewer {
        span_list_state, ..
    } = &app.screen
    {
        assert_eq!(span_list_state.selected(), Some(2));
    }

    // Up → 1.
    app.handle_event(key(KeyCode::Up));
    if let Screen::TraceViewer {
        span_list_state, ..
    } = &app.screen
    {
        assert_eq!(span_list_state.selected(), Some(1));
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
