//! Integration tests for `App` state transitions driven by `AgentLine`/`AgentDone` events.
//!
//! These tests define the exact API contract for the `Screen::AgentRun` flow.
//! They are expected to **fail at runtime** (panics from `todo!()`) until
//! `handle_agent_line` and `handle_agent_done` are implemented in T03.
//!
//! All three tests compile against the stub scaffolding added in T01.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use assay_tui::app::{AgentRunStatus, App, Screen};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// Drive the full agent-run happy path:
/// 1. Start on `Screen::AgentRun` with `Running` status.
/// 2. Deliver two lines via `handle_agent_line`.
/// 3. Signal done via `handle_agent_done(0)`.
/// 4. Assert `lines == ["line1", "line2"]` and `status == Done { exit_code: 0 }`.
#[test]
fn agent_run_streams_lines_and_transitions_to_done() {
    let mut app = App::with_project_root(None).expect("App construction should succeed");

    app.screen = Screen::AgentRun {
        chunk_slug: "test-chunk".into(),
        lines: vec![],
        scroll_offset: 0,
        status: AgentRunStatus::Running,
    };

    app.handle_agent_line("line1".into());
    app.handle_agent_line("line2".into());
    app.handle_agent_done(0);

    match &app.screen {
        Screen::AgentRun {
            lines,
            status,
            chunk_slug,
            ..
        } => {
            assert_eq!(lines, &vec!["line1".to_string(), "line2".to_string()]);
            assert_eq!(*status, AgentRunStatus::Done { exit_code: 0 });
            assert_eq!(chunk_slug, "test-chunk");
        }
        _other => panic!("expected Screen::AgentRun, got a different screen variant"),
    }
}

/// Verify that a non-zero exit code transitions status to `Failed`.
#[test]
fn agent_run_failed_exit_code_shows_failed_status() {
    let mut app = App::with_project_root(None).expect("App construction should succeed");

    app.screen = Screen::AgentRun {
        chunk_slug: "test-chunk".into(),
        lines: vec![],
        scroll_offset: 0,
        status: AgentRunStatus::Running,
    };

    app.handle_agent_done(1);

    match &app.screen {
        Screen::AgentRun { status, .. } => {
            assert_eq!(*status, AgentRunStatus::Failed { exit_code: 1 });
        }
        _other => panic!("expected Screen::AgentRun, got a different screen variant"),
    }
}

/// Verify that pressing `r` when no project is loaded is a no-op — screen stays `NoProject`.
#[test]
fn agent_run_r_key_on_no_project_is_noop() {
    let mut app = App::with_project_root(None).expect("App construction should succeed");

    // Confirm initial screen.
    assert!(
        matches!(app.screen, Screen::NoProject),
        "expected NoProject screen on startup with no project root"
    );

    // Drive the 'r' key.
    let exiting = app.handle_event(key(KeyCode::Char('r')));

    // Should not exit and should remain on NoProject.
    assert!(!exiting, "pressing 'r' should not cause the app to exit");
    assert!(
        matches!(app.screen, Screen::NoProject),
        "screen should remain NoProject after 'r' key with no project"
    );
}
