//! App-level integration tests for the gate wizard.
//!
//! These tests drive `App::handle_event` and assert `Screen` state
//! to verify all entry points (g key, e key, slash commands) and
//! the full submit → disk write → return-to-Dashboard flow.
//!
//! Run with:
//!   cargo test -p assay-tui --test gate_wizard_app

use assay_tui::app::{App, Screen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn app_type(app: &mut App, s: &str) {
    for ch in s.chars() {
        app.handle_event(key(KeyCode::Char(ch)));
    }
}

/// Create a tempdir with the minimum `.assay/` structure for App::with_project_root.
fn setup_project() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    let assay_dir = root.join(".assay");
    std::fs::create_dir_all(assay_dir.join("milestones")).unwrap();
    std::fs::create_dir_all(assay_dir.join("specs")).unwrap();
    (tmp, root)
}

/// Drive through all gate wizard steps to create a gate named `name`.
///
/// Wizard must already be open (Screen::GateWizard) before calling this.
/// Produces a gate with one criterion (name="check", desc="", cmd="echo ok").
/// The criterion must have a cmd to pass `validate_gates_spec` validation.
fn drive_wizard_create(app: &mut App, name: &str) {
    // Step 0: gate name
    app_type(app, name);
    app.handle_event(key(KeyCode::Enter));
    // Step 1: description (skip)
    app.handle_event(key(KeyCode::Enter));
    // Step 2: extends (keep none)
    app.handle_event(key(KeyCode::Enter));
    // Step 3: includes (skip)
    app.handle_event(key(KeyCode::Enter));
    // Step 4: criteria — one criterion with a cmd so validation passes
    app_type(app, "check");
    app.handle_event(key(KeyCode::Enter)); // name
    app.handle_event(key(KeyCode::Enter)); // desc (empty)
    app_type(app, "echo ok");
    app.handle_event(key(KeyCode::Enter)); // cmd = "echo ok"
    app.handle_event(key(KeyCode::Char('n'))); // add another: no
    // Step 5: preconditions — no
    app.handle_event(key(KeyCode::Char('n')));
    // Step 6: confirm
    app.handle_event(key(KeyCode::Enter));
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Pressing 'g' on Dashboard must open Screen::GateWizard.
#[test]
fn test_g_key_opens_gate_wizard() {
    let (_tmp, root) = setup_project();
    let mut app = App::with_project_root(Some(root)).unwrap();
    assert!(
        matches!(app.screen, Screen::Dashboard),
        "must start on Dashboard"
    );

    app.handle_event(key(KeyCode::Char('g')));
    assert!(
        matches!(app.screen, Screen::GateWizard(_)),
        "pressing 'g' must open Screen::GateWizard"
    );
}

/// 'g' key with no project root (NoProject screen) must be a no-op.
#[test]
fn test_g_key_noop_without_project() {
    let mut app = App::with_project_root(None).unwrap();
    assert!(
        matches!(app.screen, Screen::NoProject),
        "must start on NoProject"
    );
    app.handle_event(key(KeyCode::Char('g')));
    assert!(
        matches!(app.screen, Screen::NoProject),
        "'g' on NoProject must not change screen"
    );
}

/// Pressing 'g' then Esc must return to Dashboard.
#[test]
fn test_cancel_returns_to_dashboard() {
    let (_tmp, root) = setup_project();
    let mut app = App::with_project_root(Some(root)).unwrap();

    app.handle_event(key(KeyCode::Char('g')));
    assert!(matches!(app.screen, Screen::GateWizard(_)));

    app.handle_event(key(KeyCode::Esc));
    assert!(
        matches!(app.screen, Screen::Dashboard),
        "Esc in wizard must return to Dashboard"
    );
}

/// Completing the wizard (submit) must write gates.toml to specs dir and return to Dashboard.
#[test]
fn test_submit_writes_gates_toml() {
    let (_tmp, root) = setup_project();
    let specs_dir = root.join(".assay").join("specs");
    let mut app = App::with_project_root(Some(root)).unwrap();

    app.handle_event(key(KeyCode::Char('g')));
    assert!(matches!(app.screen, Screen::GateWizard(_)));

    drive_wizard_create(&mut app, "my-gate");

    assert!(
        matches!(app.screen, Screen::Dashboard),
        "submit must return to Dashboard"
    );
    assert!(
        specs_dir.join("my-gate").join("gates.toml").exists(),
        "gates.toml must be written to specs/my-gate/"
    );
}

/// Submitting with a duplicate slug must show an inline error and stay in wizard.
#[test]
fn test_submit_duplicate_shows_inline_error() {
    let (_tmp, root) = setup_project();
    let mut app = App::with_project_root(Some(root)).unwrap();

    // First submission — must succeed.
    app.handle_event(key(KeyCode::Char('g')));
    drive_wizard_create(&mut app, "dup-gate");
    assert!(
        matches!(app.screen, Screen::Dashboard),
        "first must succeed"
    );

    // Second submission with same name.
    app.handle_event(key(KeyCode::Char('g')));
    drive_wizard_create(&mut app, "dup-gate");

    match &app.screen {
        Screen::GateWizard(st) => {
            assert!(
                st.error.is_some(),
                "duplicate slug must set state.error; got None"
            );
        }
        Screen::Dashboard => panic!("screen must stay in GateWizard on duplicate, not Dashboard"),
        _ => panic!("unexpected screen after duplicate submit"),
    }
}

/// /gate-wizard slash command must open gate wizard in create mode.
#[test]
fn test_slash_gate_wizard_opens_create_mode() {
    let (_tmp, root) = setup_project();
    let mut app = App::with_project_root(Some(root)).unwrap();

    // Open slash overlay.
    app.handle_event(key(KeyCode::Char('/')));
    // Type "gate-wizard".
    app_type(&mut app, "gate-wizard");
    // Press Enter to execute.
    app.handle_event(key(KeyCode::Enter));

    assert!(
        matches!(app.screen, Screen::GateWizard(_)),
        "/gate-wizard slash command must open Screen::GateWizard"
    );
    // Slash state should be cleared.
    assert!(
        app.slash_state.is_none(),
        "slash_state must be cleared after /gate-wizard"
    );
    // edit_slug must be None (create mode).
    if let Screen::GateWizard(ref st) = app.screen {
        assert_eq!(
            st.edit_slug, None,
            "/gate-wizard must open in create mode (edit_slug=None)"
        );
    }
}

/// /gate-edit <slug> slash command must open gate wizard in edit mode.
#[test]
fn test_slash_gate_edit_opens_edit_mode() {
    let (_tmp, root) = setup_project();
    let specs_dir = root.join(".assay").join("specs");
    let mut app = App::with_project_root(Some(root)).unwrap();

    // Pre-create a gate so /gate-edit can load it.
    app.handle_event(key(KeyCode::Char('g')));
    drive_wizard_create(&mut app, "edit-me");
    assert!(matches!(app.screen, Screen::Dashboard));
    assert!(specs_dir.join("edit-me").join("gates.toml").exists());

    // Now use /gate-edit edit-me.
    app.handle_event(key(KeyCode::Char('/')));
    app_type(&mut app, "gate-edit edit-me");
    app.handle_event(key(KeyCode::Enter));

    assert!(
        matches!(app.screen, Screen::GateWizard(_)),
        "/gate-edit must open Screen::GateWizard"
    );
    if let Screen::GateWizard(ref st) = app.screen {
        assert_eq!(
            st.edit_slug,
            Some("edit-me".to_string()),
            "/gate-edit must open in edit mode (edit_slug=Some)"
        );
    }
}

/// /gate-edit with empty slug must show error and keep slash overlay open.
#[test]
fn test_slash_gate_edit_empty_slug_shows_error() {
    let (_tmp, root) = setup_project();
    let mut app = App::with_project_root(Some(root)).unwrap();

    app.handle_event(key(KeyCode::Char('/')));
    app_type(&mut app, "gate-edit");
    app.handle_event(key(KeyCode::Enter));

    // Screen must remain Dashboard (not GateWizard).
    assert!(
        matches!(app.screen, Screen::Dashboard),
        "empty /gate-edit must not open wizard"
    );
    // Slash overlay should still be open with error.
    assert!(
        app.slash_state.is_some(),
        "slash_state must stay open after invalid /gate-edit"
    );
    if let Some(ref s) = app.slash_state {
        assert!(
            s.error.is_some(),
            "slash_state must have error set for empty /gate-edit"
        );
    }
}

/// 'e' key on ChunkDetail must open gate wizard in edit mode.
///
/// Uses the milestone wizard ('n' key) to create a milestone with a chunk spec.
/// The milestone wizard writes both a milestone TOML and a spec gates.toml.
/// Navigating to ChunkDetail and pressing 'e' should open the gate wizard in
/// edit mode pre-filled from the existing spec.
#[test]
fn test_e_key_opens_edit_mode_on_chunk_detail() {
    let (_tmp, root) = setup_project();
    let mut app = App::with_project_root(Some(root.clone())).unwrap();

    // Create a milestone with one chunk via the 'n' milestone wizard.
    // The wizard writes both the milestone TOML and the chunk's gates.toml.
    app.handle_event(key(KeyCode::Char('n'))); // open milestone wizard
    assert!(matches!(app.screen, Screen::Wizard(_)));

    app_type(&mut app, "E Test Milestone");
    app.handle_event(key(KeyCode::Enter)); // milestone name → slug "e-test-milestone"
    app.handle_event(key(KeyCode::Enter)); // description (empty)
    app.handle_event(key(KeyCode::Char('1')));
    app.handle_event(key(KeyCode::Enter)); // chunk count = 1
    app_type(&mut app, "E Test Chunk"); // slug → "e-test-chunk"
    app.handle_event(key(KeyCode::Enter)); // chunk name
    app_type(&mut app, "build");
    app.handle_event(key(KeyCode::Enter)); // criterion name
    app_type(&mut app, "cargo build");
    app.handle_event(key(KeyCode::Enter)); // criterion cmd
    app.handle_event(key(KeyCode::Enter)); // end criteria → Submit

    assert!(
        matches!(app.screen, Screen::Dashboard),
        "milestone wizard must return to Dashboard"
    );
    assert_eq!(app.milestones.len(), 1, "milestone must be loaded");

    // Navigate: Dashboard → MilestoneDetail → ChunkDetail.
    app.handle_event(key(KeyCode::Enter)); // open milestone detail
    assert!(matches!(app.screen, Screen::MilestoneDetail { .. }));

    app.handle_event(key(KeyCode::Enter)); // open chunk detail
    assert!(
        matches!(app.screen, Screen::ChunkDetail { .. }),
        "must be on ChunkDetail"
    );

    // Press 'e' to open gate wizard in edit mode.
    // The chunk slug is "e-test-chunk" (slugified from "E Test Chunk").
    app.handle_event(key(KeyCode::Char('e')));
    assert!(
        matches!(app.screen, Screen::GateWizard(_)),
        "'e' on ChunkDetail must open Screen::GateWizard"
    );
    if let Screen::GateWizard(ref st) = app.screen {
        assert_eq!(
            st.edit_slug,
            Some("e-test-chunk".to_string()),
            "'e' must open in edit mode (edit_slug=Some(\"e-test-chunk\"))"
        );
    }
}
