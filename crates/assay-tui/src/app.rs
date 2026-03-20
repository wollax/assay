use std::path::Path;

use assay_core::{config, history, milestone};
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
    // Preserved for future use (config display, project-aware features in S02+).
    #[allow(dead_code)]
    pub project_root: std::path::PathBuf,
    #[allow(dead_code)]
    pub config: Option<Config>,
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
            };
        }

        // Load config — silently degrade on failure.
        let cfg = config::load(&project_root).ok();

        // Load milestones — silently degrade on failure.
        let milestones = milestone::milestone_scan(&assay_dir).unwrap_or_default();

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
    match event {
        Event::Key(KeyEvent { code, .. }) => match code {
            KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => return true,
            KeyCode::Up => {
                if !app.milestones.is_empty() {
                    app.list_state.select_previous();
                }
            }
            KeyCode::Down => {
                if !app.milestones.is_empty() {
                    app.list_state.select_next();
                }
            }
            KeyCode::Enter => {} // placeholder for S02/S03 screen transition
            _ => {}
        },
        Event::Resize(..) => return false, // explicit ignore — don't quit on resize
        _ => {}
    }
    false
}

/// Render the current application state to the terminal frame.
pub fn draw(frame: &mut Frame, app: &mut App) {
    match app.screen {
        Screen::NoProject => draw_no_project(frame),
        Screen::Dashboard => draw_dashboard(frame, app),
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
    frame.render_widget(Paragraph::new(" q quit  ↑↓ navigate"), footer_area);

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
}
