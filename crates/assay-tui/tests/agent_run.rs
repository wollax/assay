//! Integration tests for the AgentRun screen and `handle_tui_event` dispatch.
//!
//! These tests drive `App` with synthetic `TuiEvent` values and assert on
//! `app.screen` state.  Some tests will remain failing until T03 implements
//! `handle_tui_event` — that is expected and intentional: the tests exist to
//! constrain and verify the T03 implementation precisely.
//!
//! Run with:
//!   cargo test -p assay-tui --test agent_run

use std::path::PathBuf;

use assay_tui::app::{AgentStatus, App, Screen, TuiEvent};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn key(code: KeyCode) -> crossterm::event::KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

/// Build a minimal `.assay/` project fixture with one InProgress milestone.
/// Returns (TempDir, project root, milestone slug).
fn setup_project_with_milestone(tmp: &TempDir) -> (PathBuf, String) {
    let root = tmp.path().to_path_buf();
    let assay_dir = root.join(".assay");
    std::fs::create_dir_all(assay_dir.join("milestones")).unwrap();
    std::fs::create_dir_all(assay_dir.join("specs")).unwrap();
    std::fs::write(
        assay_dir.join("config.toml"),
        "project_name = \"test-project\"\n",
    )
    .unwrap();

    let slug = "ms-agent-test".to_string();
    std::fs::write(
        assay_dir.join("milestones").join(format!("{slug}.toml")),
        format!(
            r#"name = "Agent Test"
slug = "{slug}"
status = "InProgress"
chunks = []
completed_chunks = []
"#
        ),
    )
    .unwrap();

    // Write cycle status so App picks up cycle_slug.
    std::fs::write(
        assay_dir.join("cycle.toml"),
        format!("milestone_slug = \"{slug}\"\n"),
    )
    .unwrap();

    (root, slug)
}

/// Build a minimal project fixture with NO milestones (no InProgress chunk).
fn setup_empty_project(tmp: &TempDir) -> PathBuf {
    let root = tmp.path().to_path_buf();
    let assay_dir = root.join(".assay");
    std::fs::create_dir_all(assay_dir.join("milestones")).unwrap();
    std::fs::write(
        assay_dir.join("config.toml"),
        "project_name = \"empty-project\"\n",
    )
    .unwrap();
    root
}

/// Transition `app` to `Screen::AgentRun` for the given chunk slug directly
/// by mutating `app.screen`.  This bypasses the `r` key wiring (added in T03)
/// and lets us test event handling on the AgentRun screen in isolation.
fn put_app_in_agent_run(app: &mut App, chunk_slug: &str) {
    app.screen = Screen::AgentRun {
        chunk_slug: chunk_slug.to_string(),
        lines: Vec::new(),
        scroll_offset: 0,
        status: AgentStatus::Running,
    };
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// `AgentLine` events must accumulate in `Screen::AgentRun.lines`.
///
/// After three `AgentLine` events, `lines` should contain exactly those three
/// strings in order.  This test will fail until T03 implements `handle_tui_event`.
#[test]
fn agent_line_events_accumulate_in_agent_run_screen() {
    let tmp = TempDir::new().unwrap();
    let (root, slug) = setup_project_with_milestone(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();
    put_app_in_agent_run(&mut app, &slug);

    app.handle_tui_event(TuiEvent::AgentLine("line one".to_string()));
    app.handle_tui_event(TuiEvent::AgentLine("line two".to_string()));
    app.handle_tui_event(TuiEvent::AgentLine("line three".to_string()));

    let Screen::AgentRun { lines, .. } = &app.screen else {
        panic!("expected Screen::AgentRun, got something else");
    };
    assert_eq!(
        lines.as_slice(),
        &["line one", "line two", "line three"],
        "AgentLine events must append to Screen::AgentRun.lines in order"
    );
}

/// `AgentDone { exit_code: 0 }` must transition status to `AgentStatus::Done`.
///
/// This test will fail until T03 implements `handle_tui_event`.
#[test]
fn agent_done_event_transitions_to_done_status() {
    let tmp = TempDir::new().unwrap();
    let (root, slug) = setup_project_with_milestone(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();
    put_app_in_agent_run(&mut app, &slug);

    app.handle_tui_event(TuiEvent::AgentDone { exit_code: 0 });

    let Screen::AgentRun { status, .. } = &app.screen else {
        panic!("expected Screen::AgentRun, got something else");
    };
    assert!(
        matches!(status, AgentStatus::Done { exit_code: 0 }),
        "AgentDone with exit_code 0 must set status to Done {{ exit_code: 0 }}"
    );
}

/// `AgentDone { exit_code: 1 }` must transition status to `AgentStatus::Failed`.
///
/// This test will fail until T03 implements `handle_tui_event`.
#[test]
fn agent_done_nonzero_exit_sets_failed_status() {
    let tmp = TempDir::new().unwrap();
    let (root, slug) = setup_project_with_milestone(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();
    put_app_in_agent_run(&mut app, &slug);

    app.handle_tui_event(TuiEvent::AgentDone { exit_code: 1 });

    let Screen::AgentRun { status, .. } = &app.screen else {
        panic!("expected Screen::AgentRun, got something else");
    };
    assert!(
        matches!(status, AgentStatus::Failed { exit_code: 1 }),
        "AgentDone with non-zero exit_code must set status to Failed {{ exit_code: 1 }}"
    );
}

/// Pressing `r` on the Dashboard with no InProgress milestone must be a no-op
/// (screen stays as Dashboard).
///
/// This test should pass immediately — when no active chunk is available the
/// `r` key must not transition to `Screen::AgentRun`.
#[test]
fn r_key_no_active_chunk_is_noop() {
    let tmp = TempDir::new().unwrap();
    let root = setup_empty_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    assert!(
        matches!(app.screen, Screen::Dashboard),
        "should start on Dashboard"
    );

    app.handle_event(key(KeyCode::Char('r')));

    assert!(
        matches!(app.screen, Screen::Dashboard),
        "r key with no active chunk must leave screen as Dashboard"
    );
}
