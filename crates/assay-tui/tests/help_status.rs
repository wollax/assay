//! Contract tests for the help overlay and status bar (S05).
//!
//! Tests 1, 5, and 6 pass immediately after T01 (field presence and
//! cycle_slug loading). Tests 2, 3, and 4 fail with assertion errors until
//! T02 wires the `?` key handler and help overlay rendering.
//!
//! Run with:
//!   cargo test -p assay-tui --test help_status

use std::path::PathBuf;

use assay_tui::app::App;
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

/// Create the minimal fixture project layout under `tmp` with a configurable
/// milestone status.
///
/// - `.assay/milestones/alpha.toml` — one milestone with `status = <status>`
/// - `.assay/specs/c1/gates.toml`  — one criterion
///
/// Returns the project root path (the `tmp` directory itself).
fn setup_project_with_status(tmp: &TempDir, status: &str) -> PathBuf {
    let root = tmp.path().to_path_buf();
    let assay_dir = root.join(".assay");

    std::fs::create_dir_all(assay_dir.join("milestones")).unwrap();
    std::fs::create_dir_all(assay_dir.join("specs").join("c1")).unwrap();

    std::fs::write(
        assay_dir.join("milestones").join("alpha.toml"),
        format!(
            r#"slug = "alpha"
name = "Alpha"
status = "{status}"
created_at = "2026-01-01T00:00:00Z"
updated_at = "2026-01-01T00:00:00Z"

[[chunks]]
slug = "c1"
order = 1
"#
        ),
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

// ── Tests ─────────────────────────────────────────────────────────────────────

/// `App::with_project_root` must initialize `show_help` to `false`.
#[test]
fn show_help_starts_false() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project_with_status(&tmp, "draft");
    let app = App::with_project_root(Some(root)).unwrap();
    assert!(!app.show_help, "show_help must be false on construction");
}

/// Pressing `?` must set `show_help` to `true`.
///
/// Fails until T02 wires the `?` key handler.
#[test]
fn question_mark_opens_help() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project_with_status(&tmp, "draft");
    let mut app = App::with_project_root(Some(root)).unwrap();

    app.handle_event(key(KeyCode::Char('?')));

    assert!(app.show_help, "show_help must be true after pressing '?'");
}

/// Pressing `?` twice must toggle `show_help` back to `false`.
///
/// Fails until T02 wires the `?` key handler.
#[test]
fn question_mark_again_closes_help() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project_with_status(&tmp, "draft");
    let mut app = App::with_project_root(Some(root)).unwrap();

    app.handle_event(key(KeyCode::Char('?')));
    app.handle_event(key(KeyCode::Char('?')));

    assert!(
        !app.show_help,
        "show_help must be false after pressing '?' twice"
    );
}

/// When `show_help` is `true`, pressing `Esc` must close the overlay (set
/// `show_help = false`) and must NOT quit the app (return `false`).
///
/// Fails until T02 wires the `Esc` handler for the help overlay.
#[test]
fn esc_closes_help_when_open() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project_with_status(&tmp, "draft");
    let mut app = App::with_project_root(Some(root)).unwrap();

    app.show_help = true;
    let quit = app.handle_event(key(KeyCode::Esc));

    assert!(!app.show_help, "show_help must be false after Esc");
    assert!(!quit, "Esc with help open must not quit the app");
}

/// When the milestone has `status = "draft"`, `cycle_slug` must be `None`.
#[test]
fn cycle_slug_none_for_draft_milestone() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project_with_status(&tmp, "draft");
    let app = App::with_project_root(Some(root)).unwrap();
    assert_eq!(
        app.cycle_slug, None,
        "cycle_slug must be None when no InProgress milestone exists"
    );
}

/// When the milestone has `status = "in_progress"`, `cycle_slug` must be
/// `Some("alpha")`.
#[test]
fn cycle_slug_some_for_in_progress_milestone() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project_with_status(&tmp, "in_progress");
    let app = App::with_project_root(Some(root)).unwrap();
    assert_eq!(
        app.cycle_slug,
        Some("alpha".to_string()),
        "cycle_slug must be Some(\"alpha\") when an InProgress milestone exists"
    );
}
