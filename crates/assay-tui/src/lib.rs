use assay_types::{Config, Milestone};
use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout},
    widgets::{Block, List, ListState, Paragraph},
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

/// Render the current frame based on `app.screen`.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let [content_area] = Layout::vertical([Constraint::Fill(1)]).areas(area);

    let text = match &app.screen {
        Screen::Dashboard => "Dashboard",
        Screen::MilestoneDetail => "Milestone Detail",
        Screen::ChunkDetail => "Chunk Detail",
        Screen::Wizard(_) => "Wizard",
        Screen::Settings => "Settings",
        Screen::NoProject => "No Project",
    };

    let paragraph = Paragraph::new(text)
        .block(Block::default())
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, content_area);

    // Suppress unused-import warnings for list widgets until T02/T03.
    let _ = List::new(Vec::<&str>::new());
}

/// Handle a terminal event. Returns `false` to signal quit, `true` to continue.
pub fn handle_event(app: &mut App, event: Event) -> bool {
    if let Event::Key(key) = event {
        match key.code {
            KeyCode::Char('q') => return false,
            KeyCode::Esc => {
                if matches!(app.screen, Screen::Dashboard) {
                    return false;
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
