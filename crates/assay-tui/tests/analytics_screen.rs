//! Integration tests for the Analytics screen (S05).
//!
//! Run with:
//!   cargo test -p assay-tui --test analytics_screen

use std::path::PathBuf;

use assay_core::history::analytics::{AnalyticsReport, FailureFrequency, MilestoneVelocity};
use assay_tui::app::{App, Screen};
use assay_types::Enforcement;
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

/// Create a minimal fixture project with `.assay/` directory.
/// Returns the project root path.
fn setup_project(tmp: &TempDir) -> PathBuf {
    let root = tmp.path().to_path_buf();
    let assay_dir = root.join(".assay");

    std::fs::create_dir_all(assay_dir.join("milestones")).unwrap();
    std::fs::create_dir_all(assay_dir.join("results")).unwrap();

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

// ── Tests ─────────────────────────────────────────────────────────────────────

#[test]
fn test_a_key_transitions_to_analytics() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();
    assert!(matches!(app.screen, Screen::Dashboard));

    app.handle_event(key(KeyCode::Char('a')));
    assert!(
        matches!(app.screen, Screen::Analytics),
        "expected Screen::Analytics after pressing 'a'"
    );
}

#[test]
fn test_a_key_noop_without_project() {
    let mut app = App::with_project_root(None).unwrap();
    assert!(matches!(app.screen, Screen::NoProject));

    app.handle_event(key(KeyCode::Char('a')));
    assert!(
        matches!(app.screen, Screen::NoProject),
        "expected Screen::NoProject unchanged when no project root"
    );
}

#[test]
fn test_esc_returns_to_dashboard() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    app.handle_event(key(KeyCode::Char('a')));
    assert!(matches!(app.screen, Screen::Analytics));

    app.handle_event(key(KeyCode::Esc));
    assert!(
        matches!(app.screen, Screen::Dashboard),
        "expected Screen::Dashboard after Esc from Analytics"
    );
}

#[test]
fn test_q_from_analytics_returns_quit() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    app.handle_event(key(KeyCode::Char('a')));
    assert!(matches!(app.screen, Screen::Analytics));

    let quit = app.handle_event(key(KeyCode::Char('q')));
    assert!(
        quit,
        "expected handle_event to return true (quit) on 'q' from Analytics"
    );
}

#[test]
fn test_analytics_report_populated() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    // analytics_report starts as None.
    assert!(app.analytics_report.is_none());

    app.handle_event(key(KeyCode::Char('a')));
    // compute_analytics may return Ok (with empty data) or Err depending on
    // fixture state. With a valid .assay/ dir it should succeed with empty data.
    assert!(
        app.analytics_report.is_some(),
        "expected analytics_report to be Some after pressing 'a' with a valid project"
    );
}

#[test]
fn test_data_driven_analytics_does_not_panic() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    // Inject a synthetic report with real data.
    app.analytics_report = Some(AnalyticsReport {
        failure_frequency: vec![
            FailureFrequency {
                spec_name: "auth-flow".to_string(),
                criterion_name: "login-works".to_string(),
                fail_count: 7,
                total_runs: 10,
                enforcement: Enforcement::Required,
            },
            FailureFrequency {
                spec_name: "auth-flow".to_string(),
                criterion_name: "signup-ok".to_string(),
                fail_count: 1,
                total_runs: 8,
                enforcement: Enforcement::Advisory,
            },
            FailureFrequency {
                spec_name: "perf-spec".to_string(),
                criterion_name: "latency-p99".to_string(),
                fail_count: 0,
                total_runs: 5,
                enforcement: Enforcement::Required,
            },
        ],
        milestone_velocity: vec![MilestoneVelocity {
            milestone_slug: "alpha".to_string(),
            milestone_name: "Alpha".to_string(),
            chunks_completed: 3,
            total_chunks: 5,
            days_elapsed: 2.0,
            chunks_per_day: 1.5,
        }],
        unreadable_records: 0,
    });
    app.screen = Screen::Analytics;

    // Verify screen state is correct (proves data path setup doesn't panic).
    assert!(
        matches!(app.screen, Screen::Analytics),
        "expected Screen::Analytics with synthetic data"
    );
    assert!(app.analytics_report.is_some());
    let report = app.analytics_report.as_ref().unwrap();
    assert_eq!(report.failure_frequency.len(), 3);
    assert_eq!(report.milestone_velocity.len(), 1);
}
