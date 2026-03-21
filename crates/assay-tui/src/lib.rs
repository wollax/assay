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

/// Placeholder wizard state — real implementation comes in S02.
#[derive(Default)]
pub struct WizardState {
    // placeholder — real impl in S02
}

/// The active screen in the TUI.
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
pub struct App {
    pub screen: Screen,
    pub milestones: Vec<Milestone>,
    pub list_state: ListState,
    pub project_root: Option<PathBuf>,
    pub config: Option<Config>,
    pub show_help: bool,
}

/// Render the dashboard screen: bordered list of milestones with name, status badge, progress.
fn draw_dashboard(frame: &mut Frame, milestones: &[Milestone], list_state: &mut ListState) {
    let area = frame.area();
    let [content_area] = Layout::vertical([Constraint::Fill(1)]).areas(area);

    let block = Block::bordered().title(" Assay Dashboard ");
    let inner_area = block.inner(content_area);
    frame.render_widget(block, content_area);

    // Empty-list guard: render placeholder paragraph instead of an empty List.
    // Calling render_stateful_widget with an empty list can cause a ListState panic.
    if milestones.is_empty() {
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
pub fn draw(frame: &mut Frame, app: &mut App) {
    // Use matches! to check the discriminant first so we can borrow individual
    // fields without conflicting with a pattern match on app.screen itself.
    if matches!(app.screen, Screen::Dashboard) {
        draw_dashboard(frame, &app.milestones, &mut app.list_state);
        return;
    }
    if matches!(app.screen, Screen::NoProject) {
        draw_no_project(frame);
        return;
    }

    // Remaining screens are placeholder stubs for later slices.
    let area = frame.area();
    let [content_area] = Layout::vertical([Constraint::Fill(1)]).areas(area);
    let text = match &app.screen {
        Screen::MilestoneDetail => "Milestone detail — coming in S03",
        Screen::ChunkDetail => "Chunk Detail",
        Screen::Wizard(_) => "Wizard",
        Screen::Settings => "Settings",
        Screen::Dashboard | Screen::NoProject => unreachable!(),
    };
    let paragraph = Paragraph::new(text)
        .block(Block::default())
        .alignment(Alignment::Center);
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
