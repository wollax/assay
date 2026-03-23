//! Integration tests for the slash command module (S03).
//!
//! Tests for `parse_slash_cmd`, `tab_complete` pass immediately (T01).
//! Tests for overlay interaction (`slash_key_opens_overlay`, `enter_dispatches_status_command`,
//! `esc_closes_overlay`) compile but fail until T02 wires the overlay into App.
//!
//! Run with:
//!   cargo test -p assay-tui --test slash_commands

use std::path::PathBuf;

use assay_tui::app::App;
use assay_tui::slash::{parse_slash_cmd, tab_complete, SlashCmd};
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

/// Create a fixture project with an InProgress milestone and one chunk.
///
/// Layout:
/// - `.assay/milestones/alpha.toml` — InProgress milestone with one chunk `c1`
/// - `.assay/specs/c1/gates.toml`   — one criterion
fn setup_project_with_milestone_and_chunks(tmp: &TempDir) -> PathBuf {
    let root = tmp.path().to_path_buf();
    let assay_dir = root.join(".assay");

    std::fs::create_dir_all(assay_dir.join("milestones")).unwrap();
    std::fs::create_dir_all(assay_dir.join("specs").join("c1")).unwrap();

    std::fs::write(
        assay_dir.join("milestones").join("alpha.toml"),
        r#"slug = "alpha"
name = "Alpha"
status = "in_progress"
created_at = "2026-01-01T00:00:00Z"
updated_at = "2026-01-01T00:00:00Z"

[[chunks]]
slug = "c1"
order = 1
"#,
    )
    .unwrap();

    std::fs::write(
        assay_dir.join("specs").join("c1").join("gates.toml"),
        r#"name = "c1"

[[criteria]]
name = "first-criterion"
description = "The first gate criterion"
cmd = "true"
"#,
    )
    .unwrap();

    root
}

// ── Parse tests (pass immediately) ────────────────────────────────────────────

#[test]
fn parse_known_commands() {
    assert_eq!(parse_slash_cmd("/gate-check"), Some(SlashCmd::GateCheck));
    assert_eq!(parse_slash_cmd("/status"), Some(SlashCmd::Status));
    assert_eq!(parse_slash_cmd("/next-chunk"), Some(SlashCmd::NextChunk));
    assert_eq!(parse_slash_cmd("/spec-show"), Some(SlashCmd::SpecShow));
    assert_eq!(parse_slash_cmd("/pr-create"), Some(SlashCmd::PrCreate));
    // Case-insensitive
    assert_eq!(parse_slash_cmd("/STATUS"), Some(SlashCmd::Status));
    // Without leading slash
    assert_eq!(parse_slash_cmd("status"), Some(SlashCmd::Status));
    // With whitespace
    assert_eq!(parse_slash_cmd("  /status  "), Some(SlashCmd::Status));
}

#[test]
fn parse_unknown_returns_none() {
    assert_eq!(parse_slash_cmd("/foobar"), None);
    assert_eq!(parse_slash_cmd(""), None);
    assert_eq!(parse_slash_cmd("/"), None);
}

// ── Tab completion tests (pass immediately) ───────────────────────────────────

#[test]
fn tab_completes_partial_input() {
    // "sta" uniquely matches "status"
    assert_eq!(tab_complete("sta"), Some("/status".to_string()));
    // "g" matches "gate-check" (first alphabetically)
    assert_eq!(tab_complete("g"), Some("/gate-check".to_string()));
    // "pr" matches "pr-create"
    assert_eq!(tab_complete("pr"), Some("/pr-create".to_string()));
    // Empty input returns None
    assert_eq!(tab_complete(""), None);
    // No match
    assert_eq!(tab_complete("xyz"), None);
}

// ── Overlay interaction tests (fail until T02) ────────────────────────────────

/// Pressing `/` on the Dashboard should open the slash overlay.
#[test]
fn slash_key_opens_overlay() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project_with_milestone_and_chunks(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    app.handle_event(key(KeyCode::Char('/')));

    assert!(app.slash_state.is_some(), "slash overlay must open on '/' key");
    assert_eq!(app.slash_state.as_ref().unwrap().input, "");
}

/// Typing `status` and pressing Enter should dispatch the command and
/// populate the result field.
#[test]
fn enter_dispatches_status_command() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project_with_milestone_and_chunks(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    // Open overlay, type command, press Enter
    app.handle_event(key(KeyCode::Char('/')));
    for ch in "status".chars() {
        app.handle_event(key(KeyCode::Char(ch)));
    }
    app.handle_event(key(KeyCode::Enter));

    let slash = app.slash_state.as_ref().expect("overlay should still be open after Enter");
    assert!(slash.result.is_some(), "result should be populated after dispatching /status");
    let result = slash.result.as_ref().unwrap();
    assert!(result.contains("Milestone:"), "result should contain milestone info, got: {result}");
}

/// Pressing Esc should close the overlay.
#[test]
fn esc_closes_overlay() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project_with_milestone_and_chunks(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    // Open overlay
    app.handle_event(key(KeyCode::Char('/')));
    assert!(app.slash_state.is_some(), "overlay should be open");

    // Close it
    app.handle_event(key(KeyCode::Esc));
    assert!(app.slash_state.is_none(), "overlay should be closed after Esc");
}
