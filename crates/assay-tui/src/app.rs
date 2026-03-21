//! Application state, screen dispatch, and renderers for assay-tui.
//!
//! `App` owns all mutable state. `Screen` drives the render/event dispatch.
//! Both are public so integration tests can construct `App` directly and
//! assert on `app.screen` after driving key events.

use std::path::PathBuf;

use assay_core::milestone::milestone_scan;
use assay_core::wizard::create_from_inputs;
use assay_types::{Milestone, MilestoneStatus};
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

use crate::wizard::{WizardAction, WizardState, draw_wizard, handle_wizard_event};

// ── Screen variants ───────────────────────────────────────────────────────────

/// The active screen the application is rendering.
pub enum Screen {
    /// No `.assay/` directory found; show an info message.
    NoProject,
    /// The milestone dashboard list.
    Dashboard,
    /// The in-TUI authoring wizard.
    Wizard(WizardState),
    /// Milestone data failed to load at startup or after a wizard submit reload.
    /// Displays the error message; exits on `q` / `Esc`.
    LoadError(String),
}

// ── App ───────────────────────────────────────────────────────────────────────

/// Application state.
pub struct App {
    /// Currently rendered screen.
    pub screen: Screen,
    /// Loaded milestones (empty when no project or no milestones).
    pub milestones: Vec<Milestone>,
    /// List widget selection state for the dashboard.
    pub list_state: ListState,
    /// Path to the project root (parent of `.assay/`). `None` when no project found.
    pub project_root: Option<PathBuf>,
}

impl App {
    /// Construct an `App`, discover the project root, and pre-load milestones.
    pub fn new() -> color_eyre::Result<Self> {
        Self::with_project_root(find_project_root())
    }

    /// Construct an `App` with a known project root.
    ///
    /// Pass `Some(root)` to point at a specific directory (useful for tests).
    /// Pass `None` to start on the `NoProject` screen.
    pub fn with_project_root(project_root: Option<PathBuf>) -> color_eyre::Result<Self> {
        let (screen, milestones) = if let Some(ref root) = project_root {
            let assay_dir = root.join(".assay");
            match milestone_scan(&assay_dir) {
                Ok(milestones) => (Screen::Dashboard, milestones),
                Err(e) => {
                    let msg = format!(
                        "Could not read milestones from {}: {e}\n\
                         Check file permissions and TOML syntax in .assay/milestones/",
                        assay_dir.display()
                    );
                    (Screen::LoadError(msg), vec![])
                }
            }
        } else {
            (Screen::NoProject, vec![])
        };

        let mut list_state = ListState::default();
        if !milestones.is_empty() {
            list_state.select(Some(0));
        }

        Ok(App {
            screen,
            milestones,
            list_state,
            project_root,
        })
    }

    /// Draw the current screen into `frame`.
    pub fn draw(&mut self, frame: &mut ratatui::Frame) {
        match &self.screen {
            Screen::NoProject => draw_no_project(frame),
            Screen::Dashboard => draw_dashboard(frame, &self.milestones, &mut self.list_state),
            Screen::Wizard(state) => draw_wizard(frame, state),
            Screen::LoadError(msg) => draw_load_error(frame, msg),
        }
    }

    /// Handle a single key event. Returns `true` if the app should exit.
    pub fn handle_event(&mut self, key: crossterm::event::KeyEvent) -> bool {
        match self.screen {
            Screen::NoProject | Screen::LoadError(_) => {
                // q / Esc exit; all other keys are ignored.
                matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
            }

            Screen::Dashboard => {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return true,
                    KeyCode::Down => {
                        let i = self
                            .list_state
                            .selected()
                            .map(|s| (s + 1).min(self.milestones.len().saturating_sub(1)))
                            .unwrap_or(0);
                        self.list_state.select(Some(i));
                    }
                    KeyCode::Up => {
                        let i = self
                            .list_state
                            .selected()
                            .map(|s| s.saturating_sub(1))
                            .unwrap_or(0);
                        self.list_state.select(Some(i));
                    }
                    KeyCode::Char('n') => {
                        // project_root is always Some when Screen::Dashboard is active
                        // (see with_project_root); the guard is a defensive belt-and-suspenders
                        // check in case a future refactor reuses the Dashboard arm.
                        if self.project_root.is_some() {
                            self.screen = Screen::Wizard(WizardState::new());
                        }
                    }
                    _ => {}
                }
                false
            }

            Screen::Wizard(ref mut state) => {
                match handle_wizard_event(state, key) {
                    WizardAction::Continue => {} // state already mutated
                    WizardAction::Cancel => {
                        self.screen = Screen::Dashboard;
                    }
                    WizardAction::Submit(inputs) => {
                        // Graceful project_root guard — wizard only opens when project_root
                        // is Some ('n' keybinding guards this), but we don't rely on that
                        // convention holding across future refactors.
                        let assay_dir = match &self.project_root {
                            Some(root) => root.join(".assay"),
                            None => {
                                if let Screen::Wizard(ref mut st) = self.screen {
                                    st.error = Some(
                                        "Cannot create milestone: no project root found. \
                                         This is a bug."
                                            .to_string(),
                                    );
                                } else {
                                    unreachable!(
                                        "must be in wizard screen to receive Submit action"
                                    );
                                }
                                return false;
                            }
                        };
                        let specs_dir = assay_dir.join("specs");
                        match create_from_inputs(&inputs, &assay_dir, &specs_dir) {
                            Ok(_) => {
                                // Reload milestones; if reload fails, stay in wizard with
                                // a clear message so the user knows their milestone was
                                // written but the dashboard couldn't refresh.
                                match milestone_scan(&assay_dir) {
                                    Ok(loaded) => {
                                        self.milestones = loaded;
                                        let idx = self
                                            .milestones
                                            .iter()
                                            .position(|m| m.slug == inputs.slug)
                                            .unwrap_or(0);
                                        self.list_state.select(Some(idx));
                                        self.screen = Screen::Dashboard;
                                    }
                                    Err(e) => {
                                        if let Screen::Wizard(ref mut st) = self.screen {
                                            st.error = Some(format!(
                                                "Milestone created but failed to reload \
                                                 dashboard: {e}"
                                            ));
                                        } else {
                                            unreachable!(
                                                "must be in wizard screen to receive Submit action"
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                // Stay in wizard; show error inline.
                                if let Screen::Wizard(ref mut st) = self.screen {
                                    st.error = Some(e.to_string());
                                } else {
                                    unreachable!(
                                        "must be in wizard screen to receive Submit action"
                                    );
                                }
                            }
                        }
                    }
                }
                false
            }
        }
    }
}

// ── Screen renderers ──────────────────────────────────────────────────────────

/// Render a load-error screen when milestone data could not be read.
fn draw_load_error(frame: &mut ratatui::Frame, msg: &str) {
    let area = frame.area();
    let text = Paragraph::new(vec![
        Line::from("Failed to load project data.").bold().centered(),
        Line::from("").centered(),
        Line::from(msg).centered(),
        Line::from("").centered(),
        Line::from("Press q or Esc to exit.").centered().dim(),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Assay TUI "));
    frame.render_widget(text, area);
}

/// Render the no-project info screen.
fn draw_no_project(frame: &mut ratatui::Frame) {
    let area = frame.area();
    let msg = Paragraph::new(vec![
        Line::from("Not an Assay project.").bold().centered(),
        Line::from("Run `assay init` first, then relaunch assay-tui.")
            .centered()
            .dim(),
        Line::from("Press q or Esc to exit.").centered().dim(),
    ])
    .block(Block::default().borders(Borders::ALL).title(" Assay TUI "));
    frame.render_widget(msg, area);
}

/// Render the milestone dashboard list.
fn draw_dashboard(
    frame: &mut ratatui::Frame,
    milestones: &[Milestone],
    list_state: &mut ListState,
) {
    let area = frame.area();

    let [title_area, list_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // Title bar.
    let title = Paragraph::new(Line::from(" Assay — Milestones ").bold());
    frame.render_widget(title, title_area);

    // Milestone list.
    if milestones.is_empty() {
        let msg = Paragraph::new(Line::from("No milestones yet — press n to create one.").dim())
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(msg, list_area);
    } else {
        let items: Vec<ListItem> = milestones
            .iter()
            .map(|m| {
                // Use Display-friendly labels rather than `{:?}` Rust variant names.
                let status = match m.status {
                    MilestoneStatus::Draft => "Draft",
                    MilestoneStatus::InProgress => "In Progress",
                    MilestoneStatus::Verify => "Verify",
                    MilestoneStatus::Complete => "✓ Done",
                };
                ListItem::new(Line::from(format!("  {status:<12}  {}", m.name)))
            })
            .collect();
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(Style::default().bold().reversed());
        frame.render_stateful_widget(list, list_area, list_state);
    }

    // Hint bar.
    let hint = Paragraph::new(Line::from("↑↓ navigate · n new milestone · q quit").dim());
    frame.render_widget(hint, hint_area);
}

// ── Project discovery ─────────────────────────────────────────────────────────

/// Walk from the current directory upward looking for a `.assay/` directory.
/// Returns the directory that *contains* `.assay/`, or `None`.
fn find_project_root() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    let mut dir = cwd.as_path();
    loop {
        if dir.join(".assay").is_dir() {
            return Some(dir.to_path_buf());
        }
        dir = dir.parent()?;
    }
}
