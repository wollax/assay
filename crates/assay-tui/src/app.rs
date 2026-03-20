use assay_core::config;
use assay_core::milestone;
use assay_types::{Config, Milestone};
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Constraint, Layout},
    widgets::{Block, List, ListItem, ListState, Paragraph},
};

/// Top-level navigation state for the TUI.
pub enum Screen {
    /// Showing the milestone dashboard loaded from `.assay/`.
    Dashboard,
    /// No `.assay/` directory found — show a diagnostic message.
    NoProject,
}

/// Aggregated gate pass/fail counts for a single chunk or milestone.
pub struct GateSummary {
    pub passed: u32,
    pub failed: u32,
}

/// Central application state.
pub struct App {
    pub screen: Screen,
    pub milestones: Vec<Milestone>,
    pub gate_data: Vec<(String, GateSummary)>,
    pub list_state: ListState,
    pub project_root: std::path::PathBuf,
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

        let mut list_state = ListState::default();
        if !milestones.is_empty() {
            list_state.select(Some(0));
        }

        App {
            screen: Screen::Dashboard,
            milestones,
            gate_data: vec![],
            list_state,
            project_root,
            config: cfg,
        }
    }
}

/// Run the TUI event loop.
///
/// Returns when the user quits (e.g. presses `q`).
pub fn run(mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut app = App::new(cwd);

    loop {
        terminal.draw(|frame| draw(frame, &mut app))?;

        let ev = event::read()?;
        if handle_event(&mut app, &ev) {
            break;
        }
    }
    Ok(())
}

/// Handle a single terminal event.
///
/// Returns `true` when the application should quit.
pub fn handle_event(app: &mut App, event: &Event) -> bool {
    if let Event::Key(KeyEvent { code, .. }) = event {
        match code {
            KeyCode::Char('q') => return true,
            KeyCode::Up => {
                let len = app.milestones.len();
                if len > 0 {
                    let i = match app.list_state.selected() {
                        Some(i) => {
                            if i == 0 {
                                len - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    app.list_state.select(Some(i));
                }
            }
            KeyCode::Down => {
                let len = app.milestones.len();
                if len > 0 {
                    let i = match app.list_state.selected() {
                        Some(i) => (i + 1) % len,
                        None => 0,
                    };
                    app.list_state.select(Some(i));
                }
            }
            _ => {}
        }
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
    let msg = Paragraph::new("Not an Assay project — no .assay/ directory found.\nPress q to quit.")
        .block(Block::bordered().title("Assay TUI"));
    frame.render_widget(msg, area);
}

fn draw_dashboard(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let [header_area, list_area] =
        Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).areas(area);

    let header = Paragraph::new("Milestones  (↑↓ navigate · q quit)");
    frame.render_widget(header, header_area);

    let items: Vec<ListItem> = app
        .milestones
        .iter()
        .map(|m| {
            let chunks_total = m.chunks.len();
            let chunks_done = m.completed_chunks.len();
            let label = format!(
                "{:<30}  {:?}  {}/{}",
                m.name, m.status, chunks_done, chunks_total
            );
            ListItem::new(label)
        })
        .collect();

    let list = List::new(items)
        .block(Block::bordered().title("Milestones"))
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, list_area, &mut app.list_state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn make_key_event(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::empty()))
    }

    fn make_milestone(name: &str) -> Milestone {
        use assay_types::MilestoneStatus;
        use chrono::Utc;
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
        app.list_state.select(Some(0));
        // Force screen to Dashboard so the list has items.
        app.screen = Screen::Dashboard;

        let quit = handle_event(&mut app, &make_key_event(KeyCode::Down));
        assert!(!quit, "Down should not quit");
        let sel = app.list_state.selected().expect("should have a selection");
        assert_eq!(sel, 1, "Down from index 0 should move to index 1");
    }
}
