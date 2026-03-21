//! Contract tests for the MilestoneDetail and ChunkDetail screens.
//!
//! These tests define the behavioral contract that T02 (MilestoneDetail) and
//! T03 (ChunkDetail) must satisfy. They compile and run at T01, but all
//! navigation-dependent assertions fail until the navigation logic is wired
//! in T02/T03.
//!
//! Run with:
//!   cargo test -p assay-tui --test spec_browser

use std::path::PathBuf;

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

/// Create the minimal fixture project layout under `tmp`:
///
/// - `.assay/milestones/alpha.toml` — one milestone, one chunk ref (`c1`)
/// - `.assay/specs/c1/gates.toml`  — two criteria
///
/// Returns the project root path (the `tmp` directory itself).
fn setup_project(tmp: &TempDir) -> PathBuf {
    let root = tmp.path().to_path_buf();
    let assay_dir = root.join(".assay");

    std::fs::create_dir_all(assay_dir.join("milestones")).unwrap();
    std::fs::create_dir_all(assay_dir.join("specs").join("c1")).unwrap();

    // Minimal milestone with one chunk ref.
    // `created_at` and `updated_at` are required (non-optional) fields.
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

    // Minimal GatesSpec with two criteria.
    std::fs::write(
        assay_dir.join("specs").join("c1").join("gates.toml"),
        r#"name = "c1"

[[criteria]]
name = "first-criterion"
description = "The first gate criterion"

[[criteria]]
name = "second-criterion"
description = "The second gate criterion"
"#,
    )
    .unwrap();

    root
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Pressing Enter on the Dashboard should navigate to the MilestoneDetail screen
/// for the selected milestone (slug "alpha").
///
/// Fails until T02 wires the Enter key on Dashboard.
#[test]
fn enter_on_dashboard_navigates_to_milestone_detail() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    assert!(
        matches!(app.screen, Screen::Dashboard),
        "app must start on Dashboard"
    );

    app.handle_event(key(KeyCode::Enter));

    assert!(
        matches!(app.screen, Screen::MilestoneDetail { ref slug } if slug == "alpha"),
        "Enter on Dashboard must navigate to MilestoneDetail with slug 'alpha'; got {:?}",
        std::mem::discriminant(&app.screen),
    );
}

/// Down then Up in MilestoneDetail must move and restore chunk selection.
///
/// Fails until T02 wires navigation and loads detail_list_state.
#[test]
fn up_down_in_milestone_detail() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);

    // Add a second chunk to the milestone so there is something to move between.
    let milestone_path = root
        .join(".assay")
        .join("milestones")
        .join("alpha.toml");
    let content = std::fs::read_to_string(&milestone_path).unwrap();
    std::fs::write(
        &milestone_path,
        format!(
            "{}\n[[chunks]]\nslug = \"c2\"\norder = 2\n",
            content
        ),
    )
    .unwrap();

    let mut app = App::with_project_root(Some(root)).unwrap();

    // Navigate to MilestoneDetail via Enter on Dashboard.
    app.handle_event(key(KeyCode::Enter));

    assert!(
        matches!(app.screen, Screen::MilestoneDetail { .. }),
        "must be on MilestoneDetail after Enter; got {:?}",
        std::mem::discriminant(&app.screen),
    );

    let initial_selection = app.detail_list_state.selected().unwrap_or(0);

    app.handle_event(key(KeyCode::Down));
    let after_down = app.detail_list_state.selected().unwrap_or(0);
    assert!(
        after_down > initial_selection,
        "Down must move selection forward (was {initial_selection}, got {after_down})"
    );

    app.handle_event(key(KeyCode::Up));
    let after_up = app.detail_list_state.selected().unwrap_or(0);
    assert_eq!(
        after_up, initial_selection,
        "Up must restore selection to {initial_selection}"
    );
}

/// Pressing Esc in MilestoneDetail must return to the Dashboard.
///
/// Depends on Enter navigating to MilestoneDetail (fails until T02).
#[test]
fn esc_from_milestone_detail() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    // Navigate to MilestoneDetail (requires T02).
    app.handle_event(key(KeyCode::Enter));
    assert!(
        matches!(app.screen, Screen::MilestoneDetail { .. }),
        "must be on MilestoneDetail before testing Esc; got {:?}",
        std::mem::discriminant(&app.screen),
    );

    app.handle_event(key(KeyCode::Esc));
    assert!(
        matches!(app.screen, Screen::Dashboard),
        "Esc from MilestoneDetail must return to Dashboard"
    );
}

/// Pressing Enter on a chunk in MilestoneDetail must navigate to ChunkDetail
/// with the correct `milestone_slug` and `chunk_slug`.
///
/// Fails until T02 wires the Enter key in MilestoneDetail.
#[test]
fn enter_on_chunk_navigates_to_chunk_detail() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    // Navigate to MilestoneDetail (requires T02).
    app.handle_event(key(KeyCode::Enter));
    assert!(
        matches!(app.screen, Screen::MilestoneDetail { .. }),
        "must be on MilestoneDetail before testing chunk Enter; got {:?}",
        std::mem::discriminant(&app.screen),
    );

    // Navigate to ChunkDetail (requires T02).
    app.handle_event(key(KeyCode::Enter));
    assert!(
        matches!(
            app.screen,
            Screen::ChunkDetail { ref milestone_slug, ref chunk_slug }
                if milestone_slug == "alpha" && chunk_slug == "c1"
        ),
        "Enter on chunk must navigate to ChunkDetail(alpha, c1); got {:?}",
        std::mem::discriminant(&app.screen),
    );
}

/// Pressing Esc in ChunkDetail must return to MilestoneDetail with the correct slug.
///
/// Depends on full navigation chain (fails until T02/T03).
#[test]
fn esc_from_chunk_detail() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    // Navigate: Dashboard → MilestoneDetail → ChunkDetail.
    app.handle_event(key(KeyCode::Enter)); // → MilestoneDetail
    assert!(
        matches!(app.screen, Screen::MilestoneDetail { .. }),
        "must reach MilestoneDetail; got {:?}",
        std::mem::discriminant(&app.screen),
    );
    app.handle_event(key(KeyCode::Enter)); // → ChunkDetail
    assert!(
        matches!(app.screen, Screen::ChunkDetail { .. }),
        "must reach ChunkDetail; got {:?}",
        std::mem::discriminant(&app.screen),
    );

    // Esc must return to MilestoneDetail with the correct slug.
    app.handle_event(key(KeyCode::Esc));
    assert!(
        matches!(app.screen, Screen::MilestoneDetail { ref slug } if slug == "alpha"),
        "Esc from ChunkDetail must return to MilestoneDetail(alpha); got {:?}",
        std::mem::discriminant(&app.screen),
    );
}

/// After navigating to ChunkDetail for a project with no results directory,
/// `detail_spec` must be loaded (Some) and `detail_run` must be absent (None).
///
/// Fails until T02/T03 load spec and history on navigation.
#[test]
fn chunk_detail_no_history_all_pending() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    // Deliberately do NOT create `.assay/results/c1/` — no run history.
    let mut app = App::with_project_root(Some(root)).unwrap();

    // Navigate: Dashboard → MilestoneDetail → ChunkDetail.
    app.handle_event(key(KeyCode::Enter)); // → MilestoneDetail
    assert!(
        matches!(app.screen, Screen::MilestoneDetail { .. }),
        "must reach MilestoneDetail; got {:?}",
        std::mem::discriminant(&app.screen),
    );
    app.handle_event(key(KeyCode::Enter)); // → ChunkDetail
    assert!(
        matches!(app.screen, Screen::ChunkDetail { .. }),
        "must reach ChunkDetail; got {:?}",
        std::mem::discriminant(&app.screen),
    );

    assert!(
        app.detail_spec.is_some(),
        "detail_spec must be loaded from .assay/specs/c1/gates.toml"
    );
    assert!(
        app.detail_run.is_none(),
        "detail_run must be None when no results directory exists"
    );
}
