//! Integration tests for the agent run channel and App state machine.
//!
//! Tests 1–2 exercise `launch_agent_streaming` with real subprocesses and
//! should pass immediately after T01.
//!
//! Tests 3–8 drive `App::handle_agent_event` / `App::handle_agent_done` and
//! assert on `Screen::AgentRun` state.

use std::sync::mpsc;

use assay_core::pipeline::launch_agent_streaming;
use assay_tui::app::{AgentRunStatus, App, Screen};
use assay_types::AgentEvent;
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
        line_buffer: String::new(),
        scroll_offset: 0,
        status: AgentRunStatus::Running,
    };
    app
}

// ── Test 1: launch_agent_streaming delivers all lines ────────────────────────

#[test]
fn launch_agent_streaming_delivers_all_lines() {
    let (event_tx, event_rx) = mpsc::channel::<AgentEvent>();

    // Print 5 lines via echo.
    let args: Vec<String> = vec![
        "sh".to_string(),
        "-c".to_string(),
        "printf 'line1\\nline2\\nline3\\nline4\\nline5\\n'".to_string(),
    ];

    let handle = launch_agent_streaming(&args, std::path::Path::new("/tmp"), event_tx);

    // Collect all events before joining. Plain-text stdout lines become
    // synthetic TextDelta events via the relay fallback path (S03/T01).
    let lines: Vec<String> = event_rx
        .iter()
        .map(|e| match e {
            AgentEvent::TextDelta { text, block_index } => {
                assert_eq!(
                    block_index, 0,
                    "fallback TextDelta should have block_index 0"
                );
                text
            }
            other => panic!("expected TextDelta, got {other:?}"),
        })
        .collect();
    let exit_code = handle.relay.join().expect("thread panicked");

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
        let (event_tx, event_rx) = mpsc::channel::<AgentEvent>();
        let args: Vec<String> = vec!["true".to_string()];
        let handle = launch_agent_streaming(&args, std::path::Path::new("/tmp"), event_tx);
        let _: Vec<AgentEvent> = event_rx.iter().collect();
        let exit_code = handle.relay.join().expect("thread panicked");
        assert_eq!(exit_code, 0, "`true` should exit 0");
    }

    // Non-zero exit from `false`
    {
        let (event_tx, event_rx) = mpsc::channel::<AgentEvent>();
        let args: Vec<String> = vec!["false".to_string()];
        let handle = launch_agent_streaming(&args, std::path::Path::new("/tmp"), event_tx);
        let _: Vec<AgentEvent> = event_rx.iter().collect();
        let exit_code = handle.relay.join().expect("thread panicked");
        assert_ne!(exit_code, 0, "`false` should exit non-zero");
    }
}

// ── Test 3: handle_agent_event TextDelta (newline-terminated) pushes lines ───

/// TextBlock is now ignored in the TUI (WOL-366): it would duplicate lines
/// already rendered via the preceding TextDelta stream. This test replaces the
/// old `textblock_pushes_lines` test and drives the same assertion through
/// TextDelta + newlines, which is the canonical TUI rendering path.
#[test]
fn handle_agent_event_textdelta_newline_pushes_lines() {
    let mut app = app_in_agent_run("my-chunk");

    // Each "hello\n" / "world\n" flushes one complete line from the buffer.
    app.handle_agent_event(AgentEvent::TextDelta {
        text: "hello\n".into(),
        block_index: 0,
    });
    app.handle_agent_event(AgentEvent::TextDelta {
        text: "world\n".into(),
        block_index: 0,
    });

    match &app.screen {
        Screen::AgentRun { lines, .. } => {
            assert_eq!(lines.len(), 2, "expected 2 lines accumulated");
            assert_eq!(lines[0], "hello");
            assert_eq!(lines[1], "world");
        }
        _other => panic!("expected Screen::AgentRun, got a different screen variant"),
    }
}

/// TextBlock is a no-op in the TUI — verify it does NOT push any lines.
#[test]
fn handle_agent_event_textblock_is_noop_in_tui() {
    let mut app = app_in_agent_run("my-chunk");

    app.handle_agent_event(AgentEvent::TextBlock {
        text: "this should not appear".into(),
    });

    match &app.screen {
        Screen::AgentRun { lines, .. } => {
            assert!(lines.is_empty(), "TextBlock must not push lines in the TUI");
        }
        _other => panic!("expected Screen::AgentRun"),
    }
}

#[test]
fn handle_agent_event_textdelta_accumulates_across_newlines() {
    let mut app = app_in_agent_run("my-chunk");

    app.handle_agent_event(AgentEvent::TextDelta {
        text: "hello ".into(),
        block_index: 0,
    });
    match &app.screen {
        Screen::AgentRun { lines, .. } => assert!(lines.is_empty(), "no newline yet"),
        _ => panic!("expected Screen::AgentRun"),
    }

    app.handle_agent_event(AgentEvent::TextDelta {
        text: "world\n".into(),
        block_index: 0,
    });
    match &app.screen {
        Screen::AgentRun { lines, .. } => assert_eq!(lines, &vec!["hello world".to_string()]),
        _ => panic!("expected Screen::AgentRun"),
    }

    app.handle_agent_event(AgentEvent::TextDelta {
        text: "partial".into(),
        block_index: 0,
    });
    match &app.screen {
        Screen::AgentRun { lines, .. } => assert_eq!(
            lines,
            &vec!["hello world".to_string()],
            "partial still buffered"
        ),
        _ => panic!("expected Screen::AgentRun"),
    }

    app.handle_agent_event(AgentEvent::TextDelta {
        text: " line\nfinal\n".into(),
        block_index: 0,
    });
    match &app.screen {
        Screen::AgentRun { lines, .. } => assert_eq!(
            lines,
            &vec![
                "hello world".to_string(),
                "partial line".to_string(),
                "final".to_string(),
            ]
        ),
        _ => panic!("expected Screen::AgentRun"),
    }
}

#[test]
fn handle_agent_event_formats_tool_called() {
    let mut app = app_in_agent_run("c");
    app.handle_agent_event(AgentEvent::ToolCalled {
        name: "bash".into(),
        input_json: r#"{"command":"ls"}"#.into(),
    });
    match &app.screen {
        Screen::AgentRun { lines, .. } => {
            assert_eq!(lines, &vec![r#"[tool] bash: {"command":"ls"}"#.to_string()])
        }
        _ => panic!("expected Screen::AgentRun"),
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
            AgentRunStatus::Running => {
                panic!("status should not be Running after handle_agent_done(0)")
            }
            AgentRunStatus::Failed { .. } => {
                panic!("zero exit should transition to Done, not Failed")
            }
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
            AgentRunStatus::Running => {
                panic!("status should not be Running after handle_agent_done(1)")
            }
            AgentRunStatus::Done { .. } => {
                panic!("nonzero exit should transition to Failed, not Done")
            }
        },
        _ => panic!("expected Screen::AgentRun after handle_agent_done"),
    }
}

// ── Test 6: handle_agent_event caps lines at 10 000 ──────────────────────────

#[test]
fn handle_agent_event_caps_lines_at_ten_thousand() {
    let mut app = app_in_agent_run("my-chunk");

    // Drive via TextDelta (the active rendering path) — TextBlock is now a
    // no-op in the TUI (WOL-366), so using it here would never reach 10k lines.
    for i in 0..=10_000 {
        app.handle_agent_event(AgentEvent::TextDelta {
            text: format!("line {i}\n"),
            block_index: 0,
        });
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

// ── Test 7: handle_agent_event is a no-op on non-AgentRun screen ─────────────

#[test]
fn handle_agent_event_noops_on_non_agent_run_screen() {
    let mut app = App::with_project_root(None).expect("App::with_project_root failed");
    // app.screen is Screen::NoProject here — pumping events should not panic.
    app.handle_agent_event(AgentEvent::TextBlock {
        text: "should be ignored".into(),
    });
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
