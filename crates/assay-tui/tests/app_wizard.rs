//! Integration tests for `App`-level wizard submit behaviour.
//!
//! These tests verify that `App::handle_event` correctly propagates
//! `create_from_inputs` errors to the wizard inline-error field rather than
//! silently discarding them or resetting to a blank Dashboard.
//!
//! Run with:
//!   cargo test -p assay-tui --test app_wizard

use assay_tui::app::{App, Screen};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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

/// Drive `app` through a complete 2-chunk wizard sequence via `handle_event`.
/// The wizard (Screen::Wizard) must already be open before calling this.
///
/// Sequence: "Test Milestone" / "A description" / 2 chunks /
/// "Alpha Chunk" / "Criterion A" (+ cmd skip) / "Beta Chunk" / "Criterion B" (+ cmd skip)
fn drive_two_chunk_wizard(app: &mut App) {
    // Step 0 — milestone name
    app_type(app, "Test Milestone");
    app.handle_event(key(KeyCode::Enter));
    // Step 1 — description
    app_type(app, "A description");
    app.handle_event(key(KeyCode::Enter));
    // Step 2 — chunk count
    app.handle_event(key(KeyCode::Char('2')));
    app.handle_event(key(KeyCode::Enter));
    // Step 3 — chunk 0 name
    app_type(app, "Alpha Chunk");
    app.handle_event(key(KeyCode::Enter));
    // Step 4 — chunk 0 criteria
    app_type(app, "Criterion A");
    app.handle_event(key(KeyCode::Enter)); // name → cmd sub-step
    app.handle_event(key(KeyCode::Enter)); // blank cmd → skip
    app.handle_event(key(KeyCode::Enter)); // blank name → end criteria
    // Step 5 — chunk 1 name
    app_type(app, "Beta Chunk");
    app.handle_event(key(KeyCode::Enter));
    // Step 6 — chunk 1 criteria → Submit
    app_type(app, "Criterion B");
    app.handle_event(key(KeyCode::Enter)); // name → cmd sub-step
    app.handle_event(key(KeyCode::Enter)); // blank cmd → skip
    app.handle_event(key(KeyCode::Enter)); // blank name → Submit
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// First submission must succeed and return to Dashboard. A second submission
/// with the same milestone name (slug collision) must keep the wizard open with
/// `state.error` set — not silently return to an empty Dashboard.
#[test]
fn test_app_wizard_submit_slug_collision_shows_inline_error() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().to_path_buf();
    let assay_dir = root.join(".assay");
    std::fs::create_dir_all(assay_dir.join("milestones")).unwrap();
    std::fs::create_dir_all(assay_dir.join("specs")).unwrap();

    let mut app = App::with_project_root(Some(root)).unwrap();
    assert!(
        matches!(app.screen, Screen::Dashboard),
        "fresh project must start on Dashboard"
    );

    // ── First submission: must succeed ────────────────────────────────────────
    app.handle_event(key(KeyCode::Char('n')));
    assert!(
        matches!(app.screen, Screen::Wizard(_)),
        "pressing 'n' must open wizard"
    );
    drive_two_chunk_wizard(&mut app);
    assert!(
        matches!(app.screen, Screen::Dashboard),
        "first submission must return to Dashboard"
    );
    assert_eq!(app.milestones.len(), 1, "one milestone must be loaded");

    // ── Second submission with identical name: slug collision ─────────────────
    app.handle_event(key(KeyCode::Char('n')));
    drive_two_chunk_wizard(&mut app);

    // Screen must remain Wizard with an inline error — not blank Dashboard.
    match &app.screen {
        Screen::Wizard(st) => {
            assert!(
                st.error.is_some(),
                "slug collision must set state.error; got None"
            );
        }
        Screen::Dashboard => {
            panic!(
                "screen must stay Wizard after slug collision, not Dashboard — \
                 this would mislead the user into thinking their milestone was lost"
            );
        }
        _ => panic!("unexpected screen state after slug collision"),
    }
}
