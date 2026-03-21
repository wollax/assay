use assay_types::{Config, Milestone, MilestoneStatus};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout},
    style::Style,
    text::Text,
    widgets::{Block, List, ListItem, ListState, Paragraph},
};
use std::path::PathBuf;

/// Form state for the multi-step in-TUI milestone creation wizard.
#[derive(Default)]
pub struct WizardState {}

/// The active screen in the TUI.
// stub variants (ChunkDetail, Settings, Wizard) are unused until S02–S04
#[allow(dead_code)]
pub enum Screen {
    Dashboard,
    MilestoneDetail,
    ChunkDetail,
    Wizard(WizardState),
    Settings,
    NoProject,
}

/// Top-level application state.
///
/// `project_root` is the **parent of `.assay/`** (not the `.assay/` dir itself).
/// Pass `project_root` to `config::load`; pass `project_root.join(".assay")` to
/// `milestone_scan` — the two functions have different path contracts.
///
/// `config: None` means either no `.assay/config.toml` exists yet (normal on a
/// fresh project) or the file was found but failed to parse. Distinguish the two
/// via `config_error: Some(_)` (parse failure) vs `config_error: None` (not present).
pub struct App {
    pub screen: Screen,
    pub milestones: Vec<Milestone>,
    pub list_state: ListState,
    pub project_root: Option<PathBuf>,
    pub config: Option<Config>,
    pub show_help: bool,
    /// Set when `milestone_scan` returns an error. Rendered in the dashboard so
    /// the user knows their milestone data failed to load (not just "no milestones").
    pub scan_error: Option<String>,
    /// Set when `config.toml` exists but failed to parse (e.g. unknown field,
    /// malformed TOML). `None` means the file doesn't exist yet, not that config is OK.
    pub config_error: Option<String>,
}

/// Render the dashboard screen: bordered list of milestones with name, status badge, progress.
///
/// Takes `milestones` and `list_state` as separate parameters rather than `&mut App`
/// because matching on `app.screen` while simultaneously needing `&mut app.list_state`
/// for `render_stateful_widget` violates the borrow checker; callers borrow the fields
/// independently before passing them (D095).
fn draw_dashboard(
    frame: &mut Frame,
    milestones: &[Milestone],
    list_state: &mut ListState,
    scan_error: Option<&str>,
) {
    let area = frame.area();
    let [content_area] = Layout::vertical([Constraint::Fill(1)]).areas(area);

    let block = Block::bordered().title(" Assay Dashboard ");
    let inner_area = block.inner(content_area);
    frame.render_widget(block, content_area);

    // Scan error takes precedence: if milestone_scan returned an Err, show the
    // message so the user knows their data failed to load rather than silently
    // appearing as an empty project.
    if let Some(err) = scan_error {
        let paragraph = Paragraph::new(format!("Error loading milestones: {err}"))
            .style(Style::default().bold().red())
            .alignment(Alignment::Center);
        frame.render_widget(paragraph, inner_area);
        return;
    }

    // Guard: if milestones were reloaded and the list is now empty, a stale
    // selected index in ListState would be out of bounds when passed to
    // render_stateful_widget. Reset selection and render a placeholder instead.
    if milestones.is_empty() {
        list_state.select(None);
        let placeholder =
            Paragraph::new("No milestones — press n to create one").alignment(Alignment::Center);
        frame.render_widget(placeholder, inner_area);
        return;
    }

    let items: Vec<ListItem> = milestones
        .iter()
        .map(|m| {
            let badge = match m.status {
                MilestoneStatus::Draft => "Draft",
                MilestoneStatus::InProgress => "Active",
                MilestoneStatus::Verify => "Verify",
                MilestoneStatus::Complete => "Done",
            };
            let done = m.completed_chunks.len();
            let total = m.chunks.len();
            let text = format!("{}  [{}]  {}/{}", m.name, badge, done, total);
            ListItem::new(Text::raw(text))
        })
        .collect();

    let list = List::new(items).highlight_style(Style::default().reversed());
    frame.render_stateful_widget(list, inner_area, list_state);
}

/// Render the no-project screen: centered error message with quit hint.
fn draw_no_project(frame: &mut Frame) {
    let area = frame.area();
    let [content_area] = Layout::vertical([Constraint::Fill(1)]).areas(area);

    let text = "Not an Assay project — run `assay init` first\n\nPress q to quit";
    let paragraph = Paragraph::new(text)
        .style(Style::default().bold().red())
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, content_area);
}

/// Render the current frame based on `app.screen`.
///
/// Takes `&mut App` because `draw_dashboard` requires `&mut app.list_state` for
/// stateful widget rendering; no other `App` fields are mutated during draw.
pub fn draw(frame: &mut Frame, app: &mut App) {
    // Use matches! to check the discriminant first so we can borrow individual
    // fields without conflicting with a pattern match on app.screen itself.
    if matches!(app.screen, Screen::Dashboard) {
        draw_dashboard(
            frame,
            &app.milestones,
            &mut app.list_state,
            app.scan_error.as_deref(),
        );
        return;
    }
    if matches!(app.screen, Screen::NoProject) {
        draw_no_project(frame);
        return;
    }

    // Placeholder stubs — real implementations replace these in S02–S04.
    let area = frame.area();
    let [content_area] = Layout::vertical([Constraint::Fill(1)]).areas(area);
    let text = match &app.screen {
        Screen::MilestoneDetail => "Milestone detail — coming in S03",
        Screen::ChunkDetail => "Chunk detail — coming in S03",
        Screen::Wizard(_) => "Wizard — coming in S02",
        Screen::Settings => "Settings — coming in S04",
        // Dashboard and NoProject are handled above; this arm is unreachable in
        // correct operation. If reached, render a diagnostic message rather than
        // panicking inside the draw closure.
        Screen::Dashboard | Screen::NoProject => "[internal error: unexpected screen state]",
    };
    let paragraph = Paragraph::new(text).alignment(Alignment::Center);
    frame.render_widget(paragraph, content_area);
}

/// Handle a terminal event. Returns `false` to signal quit, `true` to continue.
pub fn handle_event(app: &mut App, event: Event) -> bool {
    if let Event::Key(key) = event {
        match key.code {
            KeyCode::Char('q') => return false,
            KeyCode::Esc => {
                // Esc returns to Dashboard from any non-Dashboard screen.
                // From Dashboard/NoProject it is a no-op (use q to quit).
                if !matches!(app.screen, Screen::Dashboard | Screen::NoProject) {
                    app.screen = Screen::Dashboard;
                }
            }
            KeyCode::Down => {
                if matches!(app.screen, Screen::Dashboard) && !app.milestones.is_empty() {
                    let len = app.milestones.len();
                    let new = match app.list_state.selected() {
                        None => 0,
                        Some(n) if n >= len - 1 => 0,
                        Some(n) => n + 1,
                    };
                    app.list_state.select(Some(new));
                }
            }
            KeyCode::Up => {
                if matches!(app.screen, Screen::Dashboard) && !app.milestones.is_empty() {
                    let len = app.milestones.len();
                    let new = match app.list_state.selected() {
                        None | Some(0) => len.saturating_sub(1),
                        Some(n) => n - 1,
                    };
                    app.list_state.select(Some(new));
                }
            }
            KeyCode::Enter => {
                if matches!(app.screen, Screen::Dashboard)
                    && !app.milestones.is_empty()
                    && app.list_state.selected().is_some()
                {
                    app.screen = Screen::MilestoneDetail;
                }
            }
            _ => {}
        }
    }
    true
}

/// Main run loop. Draws each frame and dispatches events until quit.
pub fn run(app: &mut App, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
    loop {
        terminal.draw(|f| draw(f, app))?;
        let event = event::read()?;
        if !handle_event(app, event) {
            break;
        }
    }
    Ok(())
}
