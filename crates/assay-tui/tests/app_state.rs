use assay_tui::{App, Screen, handle_event};
use assay_types::{Milestone, MilestoneStatus};
use chrono::Utc;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::widgets::ListState;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/// Build a synthetic keyboard event for the given key code.
fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

/// Construct a minimal `Milestone` fixture — no disk I/O required.
fn fake_milestone(slug: &str, name: &str) -> Milestone {
    let now = Utc::now();
    Milestone {
        slug: slug.to_string(),
        name: name.to_string(),
        description: None,
        status: MilestoneStatus::Draft,
        chunks: vec![],
        completed_chunks: vec![],
        depends_on: vec![],
        pr_branch: None,
        pr_base: None,
        pr_number: None,
        pr_url: None,
        created_at: now,
        updated_at: now,
    }
}

/// Build an `App` on the Dashboard screen with an optional list of milestones and selection.
fn make_app(milestones: Vec<Milestone>, selected: Option<usize>) -> App {
    let mut list_state = ListState::default();
    list_state.select(selected);
    App {
        screen: Screen::Dashboard,
        milestones,
        list_state,
        project_root: None,
        config: None,
        show_help: false,
    }
}

// ---------------------------------------------------------------------------
// Navigation tests
// ---------------------------------------------------------------------------

#[test]
fn navigate_down_increments_selection() {
    let milestones = vec![
        fake_milestone("a", "Alpha"),
        fake_milestone("b", "Beta"),
        fake_milestone("c", "Gamma"),
    ];
    let mut app = make_app(milestones, Some(0));
    handle_event(&mut app, key(KeyCode::Down));
    assert_eq!(app.list_state.selected(), Some(1));
}

#[test]
fn navigate_down_wraps_to_first() {
    let milestones = vec![
        fake_milestone("a", "Alpha"),
        fake_milestone("b", "Beta"),
        fake_milestone("c", "Gamma"),
    ];
    let last = milestones.len() - 1;
    let mut app = make_app(milestones, Some(last));
    handle_event(&mut app, key(KeyCode::Down));
    assert_eq!(app.list_state.selected(), Some(0));
}

#[test]
fn navigate_up_wraps_to_last() {
    let milestones = vec![
        fake_milestone("a", "Alpha"),
        fake_milestone("b", "Beta"),
        fake_milestone("c", "Gamma"),
    ];
    let last = milestones.len() - 1;
    let mut app = make_app(milestones, Some(0));
    handle_event(&mut app, key(KeyCode::Up));
    assert_eq!(app.list_state.selected(), Some(last));
}

// ---------------------------------------------------------------------------
// Quit test
// ---------------------------------------------------------------------------

#[test]
fn quit_returns_false_from_dashboard() {
    let mut app = make_app(vec![], None);
    let result = handle_event(&mut app, key(KeyCode::Char('q')));
    assert!(!result, "handle_event should return false on 'q'");
}

// ---------------------------------------------------------------------------
// Screen transition tests
// ---------------------------------------------------------------------------

#[test]
fn enter_on_dashboard_transitions_to_milestone_detail() {
    let milestones = vec![fake_milestone("m", "My Milestone")];
    let mut app = make_app(milestones, Some(0));
    handle_event(&mut app, key(KeyCode::Enter));
    assert!(
        matches!(app.screen, Screen::MilestoneDetail),
        "screen should be MilestoneDetail after Enter"
    );
}

#[test]
fn esc_returns_to_dashboard_from_milestone_detail() {
    let milestones = vec![fake_milestone("m", "My Milestone")];
    let mut app = make_app(milestones, Some(0));
    app.screen = Screen::MilestoneDetail;
    handle_event(&mut app, key(KeyCode::Esc));
    assert!(
        matches!(app.screen, Screen::Dashboard),
        "screen should be Dashboard after Esc from MilestoneDetail"
    );
}

// ---------------------------------------------------------------------------
// Empty-list guard test
// ---------------------------------------------------------------------------

#[test]
fn empty_milestones_does_not_change_list_state() {
    let mut app = make_app(vec![], None);
    handle_event(&mut app, key(KeyCode::Down));
    assert_eq!(
        app.list_state.selected(),
        None,
        "list_state should remain None when milestones is empty"
    );
}
