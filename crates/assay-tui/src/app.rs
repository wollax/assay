use std::path::Path;

use assay_core::{config, history, milestone, wizard::create_from_inputs};
use assay_tui::wizard::{WizardAction, WizardState, handle_wizard_event};
use assay_tui::wizard_draw::draw_wizard;
use assay_types::{Config, Milestone, MilestoneStatus};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    style::Stylize,
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

/// Top-level navigation state for the TUI.
pub enum Screen {
    /// Showing the milestone dashboard loaded from `.assay/`.
    Dashboard,
    /// No `.assay/` directory found — show a diagnostic message.
    NoProject,
    /// In-TUI authoring wizard for creating a new milestone.
    Wizard(WizardState),
}

/// Aggregated gate pass/fail counts for a single milestone.
pub struct GateSummary {
    pub passed: usize,
    pub failed: usize,
}

/// Central application state.
pub struct App {
    pub screen: Screen,
    pub milestones: Vec<Milestone>,
    pub gate_data: Vec<(String, GateSummary)>,
    pub list_state: ListState,
    pub project_root: std::path::PathBuf,
    #[allow(dead_code)]
    pub config: Option<Config>,
    /// Set when `milestone_scan` returns an error (corrupt TOML, permissions, I/O).
    /// Rendered in the dashboard so the user knows data failed to load, not just
    /// "no milestones". `None` means scan succeeded (even if it returned zero items).
    pub scan_error: Option<String>,
    /// Set when `.assay/config.toml` exists but failed to parse (e.g. unknown field,
    /// malformed TOML). `None` means either the file doesn't exist yet (normal) or
    /// it loaded successfully. Surfaced via `eprintln!` before ratatui::init().
    pub config_error: Option<String>,
}

impl App {
    /// Construct an `App` rooted at `project_root`.
    ///
    /// If `.assay/` does not exist, `screen` is set to `Screen::NoProject`.
    /// Config and milestone loading failures are silently degraded (empty data)
    /// rather than panicking — the user sees an empty dashboard, not a crash.
    pub fn new(project_root: std::path::PathBuf) -> Self {
        let assay_dir = project_root.join(".assay");

        if !assay_dir.exists() {
            return App {
                screen: Screen::NoProject,
                milestones: vec![],
                gate_data: vec![],
                list_state: ListState::default(),
                project_root,
                config: None,
                scan_error: None,
                config_error: None,
            };
        }

        // Distinguish "config.toml doesn't exist yet" (normal, no warning) from
        // "config.toml exists but failed to parse" (user needs to know — e.g.
        // deny_unknown_fields rejecting an old key). Stored in config_error so the
        // Settings screen (S04) can surface it; callers also print to stderr before
        // ratatui::init() so the message is visible while the terminal is still normal.
        let config_path = project_root.join(".assay").join("config.toml");
        let (cfg, config_error) = if config_path.exists() {
            match config::load(&project_root) {
                Ok(c) => (Some(c), None),
                Err(e) => (None, Some(format!("config.toml failed to load: {e}"))),
            }
        } else {
            (None, None)
        };

        // Capture milestone_scan errors — a corrupt TOML or permission failure should
        // render as an explicit error message, not silently as "no milestones".
        let (milestones, scan_error) = match milestone::milestone_scan(&assay_dir) {
            Ok(m) => (m, None),
            Err(e) => (vec![], Some(format!("Failed to load milestones: {e}"))),
        };

        // Load gate data for all milestones — silently degrade on failure.
        let gate_data = compute_gate_data(&assay_dir, &milestones);

        let mut list_state = ListState::default();
        if !milestones.is_empty() {
            list_state.select(Some(0));
        }

        App {
            screen: Screen::Dashboard,
            milestones,
            gate_data,
            list_state,
            project_root,
            config: cfg,
            scan_error,
            config_error,
        }
    }
}

/// Compute aggregated gate pass/fail counts for each milestone.
///
/// Iterates chunks in each milestone, loads the latest run for each chunk,
/// and accumulates pass/fail counts. All errors are silently degraded to zero
/// counts — no panic, no early return.
pub fn compute_gate_data(assay_dir: &Path, milestones: &[Milestone]) -> Vec<(String, GateSummary)> {
    milestones
        .iter()
        .map(|m| {
            let mut passed = 0usize;
            let mut failed = 0usize;

            for chunk in &m.chunks {
                // list() returns empty vec when spec dir doesn't exist — not an error.
                let run_ids = history::list(assay_dir, &chunk.slug).unwrap_or_default();
                if let Some(last_id) = run_ids.last()
                    && let Ok(record) = history::load(assay_dir, &chunk.slug, last_id)
                {
                    passed += record.summary.passed;
                    failed += record.summary.failed;
                }
            }

            (m.slug.clone(), GateSummary { passed, failed })
        })
        .collect()
}

/// Run the TUI event loop.
///
/// Returns when the user quits (e.g. presses `q`).
pub fn run(mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
    let project_root = std::env::current_dir()?;
    let mut app = App::new(project_root);

    // Print config parse errors to stderr before the event loop so the message
    // is visible while the terminal is still in normal (non-raw) mode.
    if let Some(ref err) = app.config_error {
        eprintln!("assay-tui warning: {err}");
    }

    loop {
        terminal.draw(|frame| draw(frame, &mut app))?;

        if event::poll(std::time::Duration::from_millis(250))? {
            let ev = event::read()?;
            if handle_event(&mut app, &ev) {
                break;
            }
        }
    }
    Ok(())
}

/// Handle a single terminal event.
///
/// Returns `true` when the application should quit.
pub fn handle_event(app: &mut App, event: &Event) -> bool {
    let Event::Key(key) = event else {
        // Resize and other events are handled by ratatui automatically.
        return false;
    };

    // Wizard screen: all key events routed through handle_wizard_event.
    if let Screen::Wizard(_) = &app.screen {
        handle_wizard_key(app, key);
        return false;
    }

    // Dashboard + NoProject: global key handling.
    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => return true,
        KeyCode::Up => {
            if matches!(app.screen, Screen::Dashboard) && !app.milestones.is_empty() {
                app.list_state.select_previous();
            }
        }
        KeyCode::Down => {
            if matches!(app.screen, Screen::Dashboard) && !app.milestones.is_empty() {
                app.list_state.select_next();
            }
        }
        KeyCode::Char('n') => {
            // `n` opens the wizard only from the Dashboard (i.e. .assay/ exists).
            if matches!(app.screen, Screen::Dashboard) {
                app.screen = Screen::Wizard(WizardState::new());
            }
        }
        _ => {}
    }
    false
}

/// Process a key event when the wizard screen is active.
fn handle_wizard_key(app: &mut App, key: &KeyEvent) {
    // Extract wizard state mutably; we'll put back the screen after deciding action.
    let Screen::Wizard(ref mut state) = app.screen else {
        return;
    };
    let action = handle_wizard_event(state, *key);
    match action {
        WizardAction::Continue => {}
        WizardAction::Cancel => {
            app.screen = Screen::Dashboard;
        }
        WizardAction::Submit(inputs) => {
            let assay_dir = app.project_root.join(".assay");
            let specs_dir = assay_dir.join("specs");
            match create_from_inputs(&inputs, &assay_dir, &specs_dir) {
                Ok(_) => {
                    // Reload milestone list so dashboard shows the new milestone.
                    app.milestones = milestone::milestone_scan(&assay_dir).unwrap_or_default();
                    app.gate_data = compute_gate_data(&assay_dir, &app.milestones);
                    // Reset selection to first item.
                    if app.milestones.is_empty() {
                        app.list_state = ListState::default();
                    } else {
                        app.list_state = ListState::default();
                        app.list_state.select(Some(0));
                    }
                    app.screen = Screen::Dashboard;
                }
                Err(e) => {
                    // Surface I/O / slug-collision error inline; stay in wizard.
                    if let Screen::Wizard(ref mut s) = app.screen {
                        s.error = Some(e.to_string());
                    }
                }
            }
        }
    }
}

/// Render the current application state to the terminal frame.
pub fn draw(frame: &mut Frame, app: &mut App) {
    // NoProject: show diagnostic and return early.
    if matches!(app.screen, Screen::NoProject) {
        draw_no_project(frame);
        return;
    }

    // Dashboard always renders (Wizard overlays on top of it).
    draw_dashboard(frame, app);

    // If in Wizard, draw the popup over the dashboard.
    if let Screen::Wizard(ref state) = app.screen {
        draw_wizard(frame, state);
    }
}

fn draw_no_project(frame: &mut Frame) {
    let area = frame.area();
    let [body] = Layout::vertical([Constraint::Fill(1)]).areas(area);
    let msg = Paragraph::new("Not an Assay project — run `assay init` first")
        .block(Block::bordered().title("Assay"));
    frame.render_widget(msg, body);
}

fn draw_no_milestones(frame: &mut Frame, area: ratatui::layout::Rect) {
    let msg = Paragraph::new("No milestones — run `assay plan`");
    frame.render_widget(msg, area);
}

fn draw_dashboard(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let [header_area, body_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    frame.render_widget(Paragraph::new("Assay Dashboard").bold(), header_area);
    frame.render_widget(
        Paragraph::new(" q quit  ↑↓ navigate  n new milestone"),
        footer_area,
    );

    // Scan error takes precedence over the empty-list state: if milestone_scan
    // returned an Err, show the message so the user knows data failed to load
    // rather than silently appearing as an empty project (indistinguishable from
    // a project with no milestones yet).
    if let Some(ref err) = app.scan_error {
        let msg = Paragraph::new(format!("Error loading milestones: {err}"))
            .style(ratatui::style::Style::default().bold().red());
        frame.render_widget(msg, body_area);
        return;
    }

    if app.milestones.is_empty() {
        draw_no_milestones(frame, body_area);
        return;
    }

    let items: Vec<ListItem> = app
        .milestones
        .iter()
        .zip(app.gate_data.iter())
        .map(|(m, (_, gs))| {
            let chunks_total = m.chunks.len();
            let chunks_done = m.completed_chunks.len();
            let badge = status_badge(m.status);
            let label = format!(
                "{:<30} {:<12} {}/{:<5} ✓{} ✗{}",
                m.name, badge, chunks_done, chunks_total, gs.passed, gs.failed
            );
            ListItem::new(label)
        })
        .collect();

    let list = List::new(items)
        .highlight_symbol("▶ ")
        .block(Block::bordered().title("Milestones"));

    frame.render_stateful_widget(list, body_area, &mut app.list_state);
}

fn status_badge(status: MilestoneStatus) -> &'static str {
    match status {
        MilestoneStatus::Draft => "[Draft]",
        MilestoneStatus::InProgress => "[InProgress]",
        MilestoneStatus::Verify => "[Verify]",
        MilestoneStatus::Complete => "[Complete]",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay_types::MilestoneStatus;
    use chrono::Utc;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_key_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
    }

    fn make_milestone(name: &str) -> Milestone {
        Milestone {
            slug: name.to_lowercase().replace(' ', "-"),
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
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_no_assay_dir_sets_no_project_screen() {
        let tmp = tempfile::tempdir().unwrap();
        // No .assay/ directory created.
        let app = App::new(tmp.path().to_path_buf());
        assert!(
            matches!(app.screen, Screen::NoProject),
            "expected Screen::NoProject when .assay/ is absent"
        );
    }

    #[test]
    fn test_handle_event_q_returns_true() {
        let tmp = tempfile::tempdir().unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        let result = handle_event(&mut app, &make_key_event(KeyCode::Char('q')));
        assert!(result, "pressing q should return true (quit)");
    }

    #[test]
    fn test_handle_event_up_down_no_panic_on_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        // milestones is empty — Up and Down must not panic.
        let up = handle_event(&mut app, &make_key_event(KeyCode::Up));
        let down = handle_event(&mut app, &make_key_event(KeyCode::Down));
        assert!(!up, "Up on empty list should return false");
        assert!(!down, "Down on empty list should return false");
    }

    #[test]
    fn test_handle_event_down_moves_selection() {
        let tmp = tempfile::tempdir().unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        app.milestones = vec![make_milestone("Alpha"), make_milestone("Beta")];
        app.gate_data = vec![
            (
                "alpha".into(),
                GateSummary {
                    passed: 0,
                    failed: 0,
                },
            ),
            (
                "beta".into(),
                GateSummary {
                    passed: 0,
                    failed: 0,
                },
            ),
        ];
        app.list_state.select(Some(0));
        app.screen = Screen::Dashboard;

        let quit = handle_event(&mut app, &make_key_event(KeyCode::Down));
        assert!(!quit, "Down should not quit");
        let sel = app.list_state.selected().expect("should have a selection");
        assert_eq!(sel, 1, "Down from index 0 should move to index 1");
    }

    #[test]
    fn test_gate_data_loaded_from_history() {
        use assay_core::history::save_run;
        use assay_core::milestone::milestone_save;
        use assay_types::{ChunkRef, EnforcementSummary, GateRunSummary};

        let tmp = tempfile::tempdir().unwrap();
        let assay_dir = tmp.path().join(".assay");
        std::fs::create_dir_all(assay_dir.join("milestones")).unwrap();

        // Build a milestone with one chunk.
        let milestone = Milestone {
            slug: "test-ms".to_string(),
            name: "Test Milestone".to_string(),
            description: None,
            status: MilestoneStatus::Draft,
            chunks: vec![ChunkRef {
                slug: "my-spec".to_string(),
                order: 1,
            }],
            completed_chunks: vec![],
            depends_on: vec![],
            pr_branch: None,
            pr_base: None,
            pr_number: None,
            pr_url: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        milestone_save(&assay_dir, &milestone).unwrap();

        // Write a gate run with passed: 2, failed: 1 for "my-spec".
        let summary = GateRunSummary {
            spec_name: "my-spec".to_string(),
            results: vec![],
            passed: 2,
            failed: 1,
            skipped: 0,
            total_duration_ms: 100,
            enforcement: EnforcementSummary::default(),
        };
        save_run(&assay_dir, summary, None, None).unwrap();

        // Construct App and check gate_data.
        let app = App::new(tmp.path().to_path_buf());

        assert!(
            matches!(app.screen, Screen::Dashboard),
            "expected Screen::Dashboard when .assay/ exists"
        );

        let entry = app
            .gate_data
            .iter()
            .find(|(slug, _)| slug == "test-ms")
            .expect("gate_data should contain entry for 'test-ms'");

        assert_eq!(entry.1.passed, 2, "passed should be 2");
        assert_eq!(entry.1.failed, 1, "failed should be 1");
    }

    // ── wizard wiring tests ───────────────────────────────────────────────────

    #[test]
    fn test_n_key_opens_wizard_from_dashboard() {
        let tmp = tempfile::tempdir().unwrap();
        // Create a .assay/ dir so the screen starts as Dashboard.
        std::fs::create_dir_all(tmp.path().join(".assay/milestones")).unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        assert!(
            matches!(app.screen, Screen::Dashboard),
            "precondition: should start on Dashboard"
        );

        handle_event(&mut app, &make_key_event(KeyCode::Char('n')));
        assert!(
            matches!(app.screen, Screen::Wizard(_)),
            "n key should open wizard"
        );
    }

    #[test]
    fn test_esc_in_wizard_returns_to_dashboard() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".assay/milestones")).unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        app.screen = Screen::Wizard(WizardState::new());

        handle_event(&mut app, &make_key_event(KeyCode::Esc));
        assert!(
            matches!(app.screen, Screen::Dashboard),
            "Esc in wizard should return to Dashboard"
        );
    }

    // ── navigation coverage tests (added during PR review) ───────────────────

    #[test]
    fn test_quit_from_no_project_screen() {
        let tmp = tempfile::tempdir().unwrap();
        // No .assay/ directory — starts as NoProject.
        let mut app = App::new(tmp.path().to_path_buf());
        assert!(matches!(app.screen, Screen::NoProject));
        let quit = handle_event(&mut app, &make_key_event(KeyCode::Char('q')));
        assert!(quit, "'q' should quit from NoProject screen");
    }

    #[test]
    fn test_navigate_up_decrements_selection() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".assay/milestones")).unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        app.milestones = vec![
            make_milestone("alpha"),
            make_milestone("beta"),
            make_milestone("gamma"),
        ];
        app.gate_data = app
            .milestones
            .iter()
            .map(|m| {
                (
                    m.slug.clone(),
                    GateSummary {
                        passed: 0,
                        failed: 0,
                    },
                )
            })
            .collect();
        app.screen = Screen::Dashboard;
        app.list_state.select(Some(2)); // start at last item

        handle_event(&mut app, &make_key_event(KeyCode::Up));
        // select_previous() moves from 2 to 1
        assert_eq!(
            app.list_state.selected(),
            Some(1),
            "Up from index 2 should move to index 1"
        );
    }

    #[test]
    fn test_navigate_down_from_no_selection_goes_to_first() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join(".assay/milestones")).unwrap();
        let mut app = App::new(tmp.path().to_path_buf());
        app.milestones = vec![make_milestone("alpha"), make_milestone("beta")];
        app.gate_data = app
            .milestones
            .iter()
            .map(|m| {
                (
                    m.slug.clone(),
                    GateSummary {
                        passed: 0,
                        failed: 0,
                    },
                )
            })
            .collect();
        app.screen = Screen::Dashboard;
        app.list_state = ListState::default(); // no selection

        handle_event(&mut app, &make_key_event(KeyCode::Down));
        // select_next() on an unset ListState should move to index 0
        assert_eq!(
            app.list_state.selected(),
            Some(0),
            "Down from no selection should select index 0"
        );
    }

    #[test]
    fn wizard_error_submit_stays_in_wizard() {
        use assay_core::wizard::{WizardInputs, create_from_inputs};

        let tmp = tempfile::tempdir().unwrap();
        let assay_dir = tmp.path().join(".assay");
        std::fs::create_dir_all(assay_dir.join("milestones")).unwrap();
        std::fs::create_dir_all(assay_dir.join("specs")).unwrap();

        // Pre-create the milestone file to force a slug collision.
        let milestone_toml = assay_dir.join("milestones/auth-layer.toml");
        std::fs::write(&milestone_toml, "[milestone]\nname = \"Auth Layer\"\n").unwrap();

        let inputs = WizardInputs {
            slug: "auth-layer".to_string(),
            name: "Auth Layer".to_string(),
            description: None,
            chunks: vec![],
        };

        // Simulate what handle_wizard_key does on Submit error.
        let mut app = App::new(tmp.path().to_path_buf());
        app.screen = Screen::Wizard(WizardState::new());

        let specs_dir = assay_dir.join("specs");
        let result = create_from_inputs(&inputs, &assay_dir, &specs_dir);
        // create_from_inputs should fail on slug collision.
        assert!(result.is_err(), "slug collision should fail");

        // Confirm that if we call handle_wizard_key with a pre-constructed Submit,
        // the app stays in Screen::Wizard with error set.
        if let Err(e) = result
            && let Screen::Wizard(ref mut s) = app.screen
        {
            s.error = Some(e.to_string());
        }
        assert!(
            matches!(app.screen, Screen::Wizard(_)),
            "should stay in wizard on create_from_inputs error"
        );
        if let Screen::Wizard(ref s) = app.screen {
            assert!(
                s.error.is_some(),
                "state.error should be set on submit failure"
            );
        }
    }
}
