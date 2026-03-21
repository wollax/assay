//! Integration tests for the agent run channel and App state machine.
//!
//! Tests 1–2 exercise `launch_agent_streaming` with real subprocesses and
//! should pass immediately after T01.
//!
//! Tests 3–8 drive `App::handle_agent_line` / `App::handle_agent_done` and
//! assert on `Screen::AgentRun` state. These are written RED — they will
//! fail until T02 implements the real state machine methods.

use std::sync::mpsc;

use assay_core::pipeline::launch_agent_streaming;
use assay_tui::app::{AgentRunStatus, App, Screen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// Construct a minimal `App` on `Screen::AgentRun` with `Running` status.
fn app_in_agent_run(chunk_slug: &str) -> App {
    let mut app = App::with_project_root(None).expect("App::with_project_root failed");
    app.screen = Screen::AgentRun {
        chunk_slug: chunk_slug.to_string(),
        lines: Vec::new(),
        scroll_offset: 0,
        status: AgentRunStatus::Running,
    };
    app
}

// ── Test 1: launch_agent_streaming delivers all lines ────────────────────────

#[test]
fn launch_agent_streaming_delivers_all_lines() {
    let (line_tx, line_rx) = mpsc::channel::<String>();

    // Print 5 lines via echo.
    let args: Vec<String> = vec![
        "sh".to_string(),
        "-c".to_string(),
        "printf 'line1\\nline2\\nline3\\nline4\\nline5\\n'".to_string(),
    ];

    let handle = launch_agent_streaming(&args, std::path::Path::new("/tmp"), line_tx);

    // Collect all lines before joining.
    let lines: Vec<String> = line_rx.iter().collect();
    let exit_code = handle.join().expect("thread panicked");

    assert_eq!(exit_code, 0, "expected exit code 0");
    assert_eq!(
        lines,
        vec!["line1", "line2", "line3", "line4", "line5"],
        "expected all 5 lines in order"
    );
}

// ── Test 2: launch_agent_streaming delivers exit code ────────────────────────

#[test]
fn launch_agent_streaming_delivers_exit_code() {
    // Zero exit from `true`
    {
        let (line_tx, line_rx) = mpsc::channel::<String>();
        let args: Vec<String> = vec!["true".to_string()];
        let handle = launch_agent_streaming(&args, std::path::Path::new("/tmp"), line_tx);
        let _: Vec<String> = line_rx.iter().collect();
        let exit_code = handle.join().expect("thread panicked");
        assert_eq!(exit_code, 0, "`true` should exit 0");
    }

    // Non-zero exit from `false`
    {
        let (line_tx, line_rx) = mpsc::channel::<String>();
        let args: Vec<String> = vec!["false".to_string()];
        let handle = launch_agent_streaming(&args, std::path::Path::new("/tmp"), line_tx);
        let _: Vec<String> = line_rx.iter().collect();
        let exit_code = handle.join().expect("thread panicked");
        assert_ne!(exit_code, 0, "`false` should exit non-zero");
    }
}

// ── Test 3: handle_agent_line accumulates in AgentRun screen ─────────────────

#[test]
fn handle_agent_line_accumulates_in_agent_run_screen() {
    let mut app = app_in_agent_run("my-chunk");

    app.handle_agent_line("hello".to_string());
    app.handle_agent_line("world".to_string());

    match &app.screen {
        Screen::AgentRun { lines, .. } => {
            assert_eq!(lines.len(), 2, "expected 2 lines accumulated");
            assert_eq!(lines[0], "hello");
            assert_eq!(lines[1], "world");
        }
        _other => panic!("expected Screen::AgentRun, got a different screen variant"),
    }
}

// ── Test 4: handle_agent_done zero exit transitions to Done ──────────────────

#[test]
fn handle_agent_done_zero_exit_transitions_to_done() {
    let mut app = app_in_agent_run("my-chunk");

    app.handle_agent_done(0);

    match &app.screen {
        Screen::AgentRun { status, .. } => match status {
            AgentRunStatus::Done { exit_code } => {
                assert_eq!(*exit_code, 0, "expected Done with exit_code 0");
            }
            AgentRunStatus::Running => panic!("status should not be Running after handle_agent_done(0)"),
            AgentRunStatus::Failed { .. } => panic!("zero exit should transition to Done, not Failed"),
        },
        _ => panic!("expected Screen::AgentRun after handle_agent_done"),
    }
}

// ── Test 5: handle_agent_done nonzero exit transitions to Failed ──────────────

#[test]
fn handle_agent_done_nonzero_exit_transitions_to_failed() {
    let mut app = app_in_agent_run("my-chunk");

    app.handle_agent_done(1);

    match &app.screen {
        Screen::AgentRun { status, .. } => match status {
            AgentRunStatus::Failed { exit_code } => {
                assert_eq!(*exit_code, 1, "expected Failed with exit_code 1");
            }
            AgentRunStatus::Running => panic!("status should not be Running after handle_agent_done(1)"),
            AgentRunStatus::Done { .. } => panic!("nonzero exit should transition to Failed, not Done"),
        },
        _ => panic!("expected Screen::AgentRun after handle_agent_done"),
    }
}

// ── Test 6: handle_agent_line caps at 10 000 ─────────────────────────────────

#[test]
fn handle_agent_line_caps_at_ten_thousand() {
    let mut app = app_in_agent_run("my-chunk");

    for i in 0..=10_000 {
        app.handle_agent_line(format!("line {i}"));
    }

    match &app.screen {
        Screen::AgentRun { lines, .. } => {
            assert_eq!(
                lines.len(),
                10_000,
                "lines should be capped at 10 000, got {}",
                lines.len()
            );
        }
        _ => panic!("expected Screen::AgentRun"),
    }
}

// ── Test 7: handle_agent_line is a no-op on non-AgentRun screen ──────────────

#[test]
fn handle_agent_line_noops_on_non_agent_run_screen() {
    // Start on Dashboard (the default when project_root is None → NoProject,
    // but let's use a real project or just assert it doesn't panic on any screen).
    let mut app = App::with_project_root(None).expect("App::with_project_root failed");
    // app.screen is Screen::NoProject here — pumping lines should not panic.
    app.handle_agent_line("should be ignored".to_string());
    // No assertion needed — the test passes if it doesn't panic.
}

// ── Test 8: r key is a no-op when event_tx is None ───────────────────────────

#[test]
fn r_key_noops_when_event_tx_is_none() {
    // Build an App on Dashboard (requires a project, but we can use tempdir).
    let dir = tempfile::tempdir().expect("tempdir failed");
    let assay_dir = dir.path().join(".assay");
    std::fs::create_dir_all(&assay_dir).expect("create_dir_all failed");
    // Seed a milestones dir so milestone_scan doesn't error.
    std::fs::create_dir_all(assay_dir.join("milestones")).expect("milestones dir");

    let mut app =
        App::with_project_root(Some(dir.path().to_path_buf())).expect("App::with_project_root");

    // Confirm we're on Dashboard and event_tx is None.
    assert!(
        matches!(app.screen, Screen::Dashboard),
        "expected Dashboard screen"
    );
    assert!(app.event_tx.is_none(), "event_tx should be None");

    // Press 'r' — should not panic and screen should remain Dashboard.
    let exit = app.handle_event(key(KeyCode::Char('r')));
    assert!(!exit, "r key should not exit");
    assert!(
        matches!(app.screen, Screen::Dashboard),
        "screen should remain Dashboard after r with no event_tx"
    );
}
