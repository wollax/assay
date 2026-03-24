//! Application state, screen dispatch, and renderers for assay-tui.
//!
//! `App` owns all mutable state. `Screen` drives the render/event dispatch.
//! Both are public so integration tests can construct `App` directly and
//! assert on `app.screen` after driving key events.

use std::path::PathBuf;
use std::sync::mpsc;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use assay_core::config::save as config_save;
use assay_core::history;
use assay_core::history::analytics::{AnalyticsReport, compute_analytics};
use assay_core::milestone::{cycle_status, milestone_load, milestone_scan};
use assay_core::pipeline::launch_agent_streaming;
use assay_core::pr::PrStatusInfo;
use assay_core::spec::{SpecEntry, load_spec_entry_with_diagnostics};
use assay_core::wizard::create_from_inputs;
use assay_types::{
    Criterion, Enforcement, GateRunRecord, GatesSpec, HarnessProfile, Milestone, MilestoneStatus,
    ProviderConfig, ProviderKind, SettingsOverride,
};
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table,
};

use crate::agent::provider_harness_writer;
use crate::mcp_panel::{AddServerForm, McpServerEntry};
use crate::slash::{
    SlashAction, SlashState, draw_slash_overlay, execute_slash_cmd, handle_slash_event,
};
use crate::wizard::{WizardAction, WizardState, draw_wizard, handle_wizard_event};

// ── TUI event channel types ───────────────────────────────────────────────────

/// Events that flow through the TUI event channel.
///
/// `Key` and `Resize` come from the crossterm input thread;
/// `AgentLine` and `AgentDone` come from the relay-wrapper thread that
/// monitors a live agent subprocess (see `launch_agent_streaming`).
pub enum TuiEvent {
    /// A keyboard event from the terminal.
    Key(crossterm::event::KeyEvent),
    /// Terminal was resized to (cols, rows).
    Resize(u16, u16),
    /// One line of stdout from the running agent subprocess.
    AgentLine(String),
    /// Agent subprocess has exited. `exit_code` is the process return value
    /// (or -1 on spawn error).
    AgentDone { exit_code: i32 },
    /// Background PR status poll result for a milestone.
    PrStatusUpdate {
        /// Milestone slug this status belongs to.
        slug: String,
        /// Polled PR status info.
        info: PrStatusInfo,
    },
}

/// Status of the agent run displayed in `Screen::AgentRun`.
pub enum AgentRunStatus {
    /// Agent subprocess is still running.
    Running,
    /// Agent exited with zero (success).
    Done { exit_code: i32 },
    /// Agent exited with a non-zero code (failure).
    Failed { exit_code: i32 },
}

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
    /// Chunk list for a single milestone. `slug` identifies the milestone.
    MilestoneDetail { slug: String },
    /// Detail view for a single chunk within a milestone.
    ChunkDetail {
        milestone_slug: String,
        chunk_slug: String,
    },
    /// Provider / model configuration screen.
    Settings {
        /// Currently selected provider in the list.
        selected: usize,
        /// Inline error message from a failed save attempt.
        error: Option<String>,
        /// Editable buffer for the planning model name.
        planning_model: String,
        /// Editable buffer for the execution model name.
        execution_model: String,
        /// Editable buffer for the review model name.
        review_model: String,
        /// Which model field has focus (`None` = provider list has focus).
        model_focus: Option<usize>,
    },
    /// Live agent run view — streams stdout lines and shows final status.
    AgentRun {
        /// Slug of the chunk being run.
        chunk_slug: String,
        /// Accumulated stdout lines from the agent subprocess (capped at 10 000).
        lines: Vec<String>,
        /// Scroll offset for the line list.
        scroll_offset: usize,
        /// Current run status.
        status: AgentRunStatus,
    },
    /// Full-screen analytics view showing gate failure frequency and milestone velocity.
    Analytics,
    /// MCP server configuration panel — add, delete, and persist servers.
    McpPanel {
        /// Loaded MCP server entries (sorted alphabetically by name).
        servers: Vec<McpServerEntry>,
        /// Currently selected server index in the list.
        selected: usize,
        /// Active add-server form, if open.
        add_form: Option<AddServerForm>,
        /// Inline error message from a failed load or save attempt.
        error: Option<String>,
    },
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
    /// List widget selection state for the chunk list in MilestoneDetail.
    pub detail_list_state: ListState,
    /// Loaded milestone data for MilestoneDetail and ChunkDetail.
    pub detail_milestone: Option<Milestone>,
    /// Loaded GatesSpec for ChunkDetail (`None` for legacy specs or on error).
    pub detail_spec: Option<GatesSpec>,
    /// Diagnostic reason when `detail_spec` is `None`.
    pub detail_spec_note: Option<String>,
    /// Latest gate run record (`None` if no history exists).
    pub detail_run: Option<GateRunRecord>,
    /// Whether the help overlay is currently visible.
    pub show_help: bool,
    /// Slug of the currently active (InProgress) milestone, or `None` when no
    /// milestone is in progress.  Used by the status bar renderer.
    pub cycle_slug: Option<String>,
    /// Loaded project config (used by status bar and settings screen).
    pub config: Option<assay_types::Config>,
    /// Sender side of the TUI event channel. `None` until the channel is wired
    /// in `main.rs`. Used by the relay-wrapper thread to push `AgentLine` and
    /// `AgentDone` events into the main loop.
    pub event_tx: Option<mpsc::Sender<TuiEvent>>,
    /// Join handle for a live agent subprocess thread. `None` when no agent
    /// is running. The thread returns the process exit code as `i32`.
    pub agent_thread: Option<std::thread::JoinHandle<i32>>,
    /// Slash command overlay state. `Some` when overlay is open.
    pub slash_state: Option<SlashState>,
    /// Analytics report computed when the user presses `a` from Dashboard.
    /// Recomputed on every `a` key press; `None` until the user first navigates to Analytics.
    pub analytics_report: Option<AnalyticsReport>,
    /// Cached PR status info per milestone slug, populated by the background
    /// polling thread via `TuiEvent::PrStatusUpdate`.
    pub pr_statuses: HashMap<String, PrStatusInfo>,
    /// Milestones with a `pr_number`, shared with the background polling
    /// thread. Each entry is `(slug, pr_number)`.
    pub poll_targets: Arc<Mutex<Vec<(String, u64)>>>,
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
        let (screen, milestones, cycle_slug) = if let Some(ref root) = project_root {
            let assay_dir = root.join(".assay");
            match milestone_scan(&assay_dir) {
                Ok(milestones) => {
                    let slug = match cycle_status(&assay_dir) {
                        Ok(status) => status.map(|cs| cs.milestone_slug),
                        Err(e) => {
                            tracing::warn!(error = %e, "Could not read cycle status");
                            None
                        }
                    };
                    (Screen::Dashboard, milestones, slug)
                }
                Err(e) => {
                    let msg = format!(
                        "Could not read milestones from {}: {e}\n\
                         Check file permissions and TOML syntax in .assay/milestones/",
                        assay_dir.display()
                    );
                    (Screen::LoadError(msg), vec![], None)
                }
            }
        } else {
            (Screen::NoProject, vec![], None)
        };

        let mut list_state = ListState::default();
        if !milestones.is_empty() {
            list_state.select(Some(0));
        }

        let config = project_root.as_deref().and_then(|root| {
            let config_path = root.join(".assay").join("config.toml");
            if config_path.exists() {
                match assay_core::config::load(root) {
                    Ok(cfg) => Some(cfg),
                    Err(e) => {
                        // Config file exists but is unreadable — surface as status-bar
                        // warning via eprintln (not silently swallowed).
                        tracing::warn!(error = %e, "Failed to load .assay/config.toml");
                        None
                    }
                }
            } else {
                None
            }
        });

        let poll_targets: Vec<(String, u64)> = milestones
            .iter()
            .filter_map(|m| m.pr_number.map(|n| (m.slug.clone(), n)))
            .collect();

        Ok(App {
            screen,
            milestones,
            list_state,
            project_root,
            detail_list_state: ListState::default(),
            detail_milestone: None,
            detail_spec: None,
            detail_spec_note: None,
            detail_run: None,
            show_help: false,
            cycle_slug,
            config,
            analytics_report: None,
            event_tx: None,
            agent_thread: None,
            slash_state: None,
            pr_statuses: HashMap::new(),
            poll_targets: Arc::new(Mutex::new(poll_targets)),
        })
    }

    /// Accumulate a line of agent stdout into `Screen::AgentRun`.
    ///
    /// No-op if the current screen is not `Screen::AgentRun`.
    /// Lines are capped at 10 000 to prevent unbounded memory growth.
    pub fn handle_agent_line(&mut self, line: String) {
        if let Screen::AgentRun { ref mut lines, .. } = self.screen {
            lines.push(line);
            if lines.len() > 10_000 {
                lines.remove(0);
            }
        }
    }

    /// Store a PR status update from the background polling thread.
    pub fn handle_pr_status_update(&mut self, slug: String, info: PrStatusInfo) {
        self.pr_statuses.insert(slug, info);
    }

    /// Refresh the shared `poll_targets` list from `self.milestones`.
    ///
    /// Called whenever `self.milestones` is reloaded from disk so the
    /// background polling thread picks up newly created PRs or drops
    /// milestones whose PR was removed.
    pub fn refresh_poll_targets(&self) {
        let targets: Vec<(String, u64)> = self
            .milestones
            .iter()
            .filter_map(|m| m.pr_number.map(|n| (m.slug.clone(), n)))
            .collect();
        if let Ok(mut guard) = self.poll_targets.lock() {
            *guard = targets;
        }
    }

    /// Handle agent subprocess exit.
    ///
    /// Transitions `Screen::AgentRun.status` to `Done` (exit 0) or `Failed`
    /// (non-zero exit). Then refreshes milestones and cycle_slug from disk so
    /// the dashboard reflects any state changes caused by the agent run.
    pub fn handle_agent_done(&mut self, exit_code: i32) {
        if let Screen::AgentRun { ref mut status, .. } = self.screen {
            *status = if exit_code == 0 {
                AgentRunStatus::Done { exit_code }
            } else {
                AgentRunStatus::Failed { exit_code }
            };
        }
        // Refresh disk state (graceful degradation on I/O error).
        if let Some(ref root) = self.project_root {
            let assay_dir = root.join(".assay");
            if let Ok(ms) = milestone_scan(&assay_dir) {
                self.milestones = ms;
                self.refresh_poll_targets();
            }
            match cycle_status(&assay_dir) {
                Ok(status) => {
                    self.cycle_slug = status.map(|cs| cs.milestone_slug);
                }
                Err(_) => {
                    // Preserve existing cycle_slug on refresh failure.
                }
            }
        }
        // Clear the handle so a subsequent `r` press can start a new run.
        self.agent_thread = None;
    }

    /// Handle key events for `Screen::McpPanel`.
    ///
    /// Extracted as a separate method to avoid borrow-splitting issues in
    /// the main `handle_event` match (D098). Returns `true` if the app
    /// should exit.
    fn handle_mcp_panel_event(&mut self, key: crossterm::event::KeyEvent) -> bool {
        let Screen::McpPanel {
            servers,
            selected: _,
            add_form,
            error,
        } = &mut self.screen
        else {
            return false;
        };

        // When add_form is active, intercept form-specific keys first.
        if let Some(form) = add_form {
            match key.code {
                KeyCode::Esc => {
                    *add_form = None;
                    *error = None;
                }
                KeyCode::Tab => {
                    form.active_field = if form.active_field == 0 { 1 } else { 0 };
                }
                KeyCode::Char(c) => {
                    if form.active_field == 0 {
                        form.name.push(c);
                    } else {
                        form.command.push(c);
                    }
                }
                KeyCode::Backspace => {
                    if form.active_field == 0 {
                        form.name.pop();
                    } else {
                        form.command.pop();
                    }
                }
                KeyCode::Enter => {
                    let new_name = form.name.trim().to_string();
                    let new_command = form.command.trim().to_string();
                    if new_name.is_empty() {
                        *error = Some("Server name cannot be empty.".to_string());
                    } else if new_command.is_empty() {
                        *error = Some("Command cannot be empty.".to_string());
                    } else if servers.iter().any(|s| s.name == new_name) {
                        *error = Some(format!("Duplicate server name: {new_name}"));
                    } else {
                        *error = None;
                        let new_entry = McpServerEntry {
                            name: new_name,
                            command: new_command,
                            args: vec![],
                        };
                        servers.push(new_entry);
                        servers.sort_by(|a, b| a.name.cmp(&b.name));
                        *add_form = None;
                    }
                }
                _ => {}
            }
            return false;
        }

        // Normal McpPanel keys (no add_form active).
        match key.code {
            KeyCode::Esc => {
                self.screen = Screen::Dashboard;
            }
            KeyCode::Char('q') => return true,
            KeyCode::Up => {
                if let Screen::McpPanel { selected, .. } = &mut self.screen {
                    *selected = selected.saturating_sub(1);
                }
            }
            KeyCode::Down => {
                if let Screen::McpPanel {
                    servers, selected, ..
                } = &mut self.screen
                    && !servers.is_empty()
                {
                    *selected = (*selected + 1).min(servers.len() - 1);
                }
            }
            KeyCode::Char('a') => {
                if let Screen::McpPanel { add_form, .. } = &mut self.screen {
                    *add_form = Some(AddServerForm::new());
                }
            }
            KeyCode::Char('d') => {
                if let Screen::McpPanel {
                    servers, selected, ..
                } = &mut self.screen
                    && !servers.is_empty()
                {
                    servers.remove(*selected);
                    if *selected >= servers.len() && !servers.is_empty() {
                        *selected = servers.len() - 1;
                    }
                }
            }
            KeyCode::Char('w') => {
                // Extract servers for save, then handle result.
                let save_result = if let Screen::McpPanel { servers, .. } = &self.screen {
                    if let Some(ref root) = self.project_root {
                        Some(crate::mcp_panel::mcp_config_save(root, servers))
                    } else {
                        Some(Err("No project root found.".to_string()))
                    }
                } else {
                    None
                };
                if let Some(result) = save_result {
                    match result {
                        Ok(()) => {
                            self.screen = Screen::Dashboard;
                        }
                        Err(e) => {
                            if let Screen::McpPanel { error, .. } = &mut self.screen {
                                *error = Some(e);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
        false
    }

    /// Draw the current screen into `frame`.
    pub fn draw(&mut self, frame: &mut ratatui::Frame) {
        let [content_area, status_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

        match &self.screen {
            Screen::NoProject => draw_no_project(frame, content_area),
            Screen::Dashboard => draw_dashboard(
                frame,
                content_area,
                &self.milestones,
                &mut self.list_state,
                &self.pr_statuses,
            ),
            Screen::Wizard(state) => draw_wizard(frame, content_area, state),
            Screen::LoadError(msg) => draw_load_error(frame, content_area, msg),
            Screen::MilestoneDetail { .. } => {
                draw_milestone_detail(
                    frame,
                    content_area,
                    self.detail_milestone.as_ref(),
                    &mut self.detail_list_state,
                );
            }
            Screen::ChunkDetail { chunk_slug, .. } => {
                let slug = chunk_slug.clone();
                draw_chunk_detail(
                    frame,
                    content_area,
                    &slug,
                    self.detail_spec.as_ref(),
                    self.detail_spec_note.as_deref(),
                    self.detail_run.as_ref(),
                );
            }
            Screen::Settings {
                selected,
                error,
                model_focus,
                planning_model,
                execution_model,
                review_model,
            } => {
                draw_settings(
                    frame,
                    content_area,
                    self.config.as_ref(),
                    *selected,
                    error.as_deref(),
                    *model_focus,
                    planning_model,
                    execution_model,
                    review_model,
                );
            }
            Screen::AgentRun {
                chunk_slug,
                lines,
                scroll_offset,
                status,
            } => {
                draw_agent_run(
                    frame,
                    content_area,
                    chunk_slug,
                    lines,
                    *scroll_offset,
                    status,
                );
            }
            Screen::Analytics => {
                draw_analytics(frame, content_area, self.analytics_report.as_ref());
            }
            Screen::McpPanel {
                servers,
                selected,
                add_form,
                error,
            } => {
                crate::mcp_panel::draw_mcp_panel(
                    frame,
                    content_area,
                    servers,
                    *selected,
                    add_form.as_ref(),
                    error.as_deref(),
                );
            }
        }

        let project_name = self
            .config
            .as_ref()
            .map(|c| c.project_name.as_str())
            .unwrap_or("");
        draw_status_bar(frame, status_area, project_name, self.cycle_slug.as_deref());

        if self.show_help {
            draw_help_overlay(frame, frame.area());
        }

        if let Some(ref state) = self.slash_state {
            draw_slash_overlay(frame, content_area, state);
        }
    }

    /// Handle a single key event. Returns `true` if the app should exit.
    pub fn handle_event(&mut self, key: crossterm::event::KeyEvent) -> bool {
        // When help overlay is visible, only ? and Esc dismiss it; all other keys are no-ops.
        if self.show_help {
            if matches!(key.code, KeyCode::Char('?') | KeyCode::Esc) {
                self.show_help = false;
            }
            return false;
        }

        // When slash overlay is visible, intercept all keys before screen dispatch (D104).
        if let Some(ref mut slash) = self.slash_state {
            match handle_slash_event(slash, key) {
                SlashAction::Continue => {}
                SlashAction::Close => {
                    self.slash_state = None;
                }
                SlashAction::Execute(cmd) => {
                    if let Some(ref root) = self.project_root {
                        let result = execute_slash_cmd(cmd, root);
                        if let Some(ref mut s) = self.slash_state {
                            s.result = Some(result);
                        }
                    } else if let Some(ref mut s) = self.slash_state {
                        s.error = Some("No project root found.".to_string());
                    }
                }
            }
            return false;
        }

        // Global ? key opens help from any non-wizard screen.
        if key.code == KeyCode::Char('?') && !matches!(self.screen, Screen::Wizard(_)) {
            self.show_help = true;
            return false;
        }

        match self.screen {
            Screen::NoProject | Screen::LoadError(_) => {
                // q / Esc exit; all other keys are ignored.
                matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
            }

            Screen::Dashboard => {
                match key.code {
                    KeyCode::Char('/') => {
                        self.slash_state = Some(SlashState::default());
                        return false;
                    }
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
                    KeyCode::Char('r') => {
                        // Guard: no-op while a relay thread is already live.
                        // Prevents double-spawning when the user presses r → Esc → r
                        // before the previous agent run has finished.
                        if self.agent_thread.is_some() {
                            return false;
                        }
                        // Guard: no-op when event_tx is None (test environments).
                        let tx = match self.event_tx.clone() {
                            Some(tx) => tx,
                            None => return false,
                        };
                        let assay_dir = match &self.project_root {
                            Some(root) => root.join(".assay"),
                            None => return false,
                        };
                        // Get the active chunk slug from cycle_status.
                        let chunk_slug = match cycle_status(&assay_dir) {
                            Ok(Some(cs)) => cs
                                .active_chunk_slug
                                .unwrap_or_else(|| cs.milestone_slug.clone()),
                            _ => return false,
                        };
                        // Build a minimal HarnessProfile for provider dispatch.
                        let profile = HarnessProfile {
                            name: chunk_slug.clone(),
                            prompt_layers: vec![],
                            settings: SettingsOverride {
                                model: None,
                                permissions: vec![],
                                tools: vec![],
                                max_turns: None,
                            },
                            hooks: vec![],
                            working_dir: None,
                        };
                        // Write harness config to a temp dir to avoid polluting worktree.
                        let run_dir =
                            std::env::temp_dir().join(format!("assay-agent-{}", chunk_slug));
                        if std::fs::create_dir_all(&run_dir).is_err() {
                            return false;
                        }
                        let writer = provider_harness_writer(self.config.as_ref());
                        let cli_args = match writer(&profile, &run_dir) {
                            Ok(args) => args,
                            Err(_) => return false,
                        };
                        let working_dir = run_dir.clone();
                        // Transition to AgentRun screen.
                        self.screen = Screen::AgentRun {
                            chunk_slug: chunk_slug.clone(),
                            lines: vec![],
                            scroll_offset: 0,
                            status: AgentRunStatus::Running,
                        };
                        // Spawn relay-wrapper thread: inner launch_agent_streaming +
                        // outer drain loop. Serializes AgentLine before AgentDone to
                        // prevent line loss.
                        let tui_tx = tx.clone();
                        let handle = std::thread::spawn(move || {
                            let (str_tx, str_rx) = std::sync::mpsc::channel::<String>();
                            let inner = launch_agent_streaming(&cli_args, &working_dir, str_tx);
                            // Drain lines → TuiEvent::AgentLine.
                            for line in str_rx {
                                let _ = tui_tx.send(TuiEvent::AgentLine(line));
                            }
                            // All lines sent; get exit code from inner thread.
                            let exit_code = inner.join().unwrap_or(-1);
                            let _ = tui_tx.send(TuiEvent::AgentDone { exit_code });
                            exit_code
                        });
                        self.agent_thread = Some(handle);
                    }
                    KeyCode::Char('m') => {
                        // Open MCP server configuration panel.
                        if let Some(ref root) = self.project_root {
                            let (servers, load_error) =
                                match crate::mcp_panel::mcp_config_load(root) {
                                    Ok(s) => (s, None),
                                    Err(e) => (Vec::new(), Some(e)),
                                };
                            self.screen = Screen::McpPanel {
                                servers,
                                selected: 0,
                                add_form: None,
                                error: load_error,
                            };
                        }
                    }
                    KeyCode::Char('a') => {
                        if let Some(ref root) = self.project_root {
                            let assay_dir = root.join(".assay");
                            self.analytics_report = compute_analytics(&assay_dir).ok();
                            self.screen = Screen::Analytics;
                        }
                    }
                    KeyCode::Char('s') => {
                        // Open settings screen.  Pre-select the current provider if
                        // config is loaded; default to Anthropic (index 0) otherwise.
                        let selected = self
                            .config
                            .as_ref()
                            .and_then(|c| c.provider.as_ref())
                            .map(|p| match p.provider {
                                ProviderKind::Anthropic => 0,
                                ProviderKind::OpenAi => 1,
                                ProviderKind::Ollama => 2,
                            })
                            .unwrap_or(0);
                        let (pm, em, rm) = self
                            .config
                            .as_ref()
                            .and_then(|c| c.provider.as_ref())
                            .map(|p| {
                                (
                                    p.planning_model.clone().unwrap_or_default(),
                                    p.execution_model.clone().unwrap_or_default(),
                                    p.review_model.clone().unwrap_or_default(),
                                )
                            })
                            .unwrap_or_default();
                        self.screen = Screen::Settings {
                            selected,
                            error: None,
                            planning_model: pm,
                            execution_model: em,
                            review_model: rm,
                            model_focus: None,
                        };
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = self.list_state.selected()
                            && let Some(ms) = self.milestones.get(idx)
                        {
                            let slug = ms.slug.clone();
                            let assay_dir = match &self.project_root {
                                Some(root) => root.join(".assay"),
                                None => return false,
                            };
                            match milestone_load(&assay_dir, &slug) {
                                Ok(loaded) => {
                                    self.detail_list_state.select(if loaded.chunks.is_empty() {
                                        None
                                    } else {
                                        Some(0)
                                    });
                                    self.detail_milestone = Some(loaded);
                                    self.screen = Screen::MilestoneDetail { slug };
                                }
                                Err(e) => {
                                    self.screen = Screen::LoadError(format!(
                                        "Failed to load milestone '{slug}': {e}"
                                    ));
                                }
                            }
                        }
                    }
                    _ => {}
                }
                false
            }

            Screen::MilestoneDetail { .. } => {
                let chunk_count = self
                    .detail_milestone
                    .as_ref()
                    .map(|m| m.chunks.len())
                    .unwrap_or(0);
                match key.code {
                    KeyCode::Char('/') => {
                        self.slash_state = Some(SlashState::default());
                        return false;
                    }
                    KeyCode::Esc => {
                        self.screen = Screen::Dashboard;
                    }
                    KeyCode::Char('q') => return true,
                    KeyCode::Down => {
                        if chunk_count > 0 {
                            let i = self
                                .detail_list_state
                                .selected()
                                .map(|s| (s + 1) % chunk_count)
                                .unwrap_or(0);
                            self.detail_list_state.select(Some(i));
                        }
                    }
                    KeyCode::Up => {
                        if chunk_count > 0 {
                            let i = self
                                .detail_list_state
                                .selected()
                                .map(|s| if s == 0 { chunk_count - 1 } else { s - 1 })
                                .unwrap_or(0);
                            self.detail_list_state.select(Some(i));
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(idx) = self.detail_list_state.selected() {
                            // Get the sorted chunk at this index (same sort as draw).
                            if let Some(milestone) = &self.detail_milestone {
                                let mut sorted_chunks = milestone.chunks.clone();
                                sorted_chunks.sort_by_key(|c| c.order);
                                if let Some(chunk) = sorted_chunks.get(idx) {
                                    let milestone_slug = milestone.slug.clone();
                                    let chunk_slug = chunk.slug.clone();
                                    let assay_dir = match &self.project_root {
                                        Some(root) => root.join(".assay"),
                                        None => return false,
                                    };
                                    let specs_dir = assay_dir.join("specs");
                                    // Load spec entry.
                                    match load_spec_entry_with_diagnostics(&chunk_slug, &specs_dir)
                                    {
                                        Ok(SpecEntry::Directory { gates, .. }) => {
                                            self.detail_spec = Some(gates);
                                            self.detail_spec_note = None;
                                        }
                                        Ok(SpecEntry::Legacy { .. }) => {
                                            self.detail_spec = None;
                                            self.detail_spec_note = Some(
                                                "Legacy flat spec — criteria not available in this view"
                                                    .to_string(),
                                            );
                                        }
                                        Err(e) => {
                                            self.detail_spec = None;
                                            self.detail_spec_note =
                                                Some(format!("Failed to load spec: {e}"));
                                        }
                                    }
                                    // Load latest gate run (empty history is not an error).
                                    self.detail_run = match history::list(&assay_dir, &chunk_slug) {
                                        Ok(ids) if !ids.is_empty() => {
                                            let run_id = ids.last().unwrap().clone();
                                            history::load(&assay_dir, &chunk_slug, &run_id).ok()
                                        }
                                        _ => None,
                                    };
                                    self.screen = Screen::ChunkDetail {
                                        milestone_slug,
                                        chunk_slug,
                                    };
                                }
                            }
                        }
                    }
                    _ => {}
                }
                false
            }

            Screen::ChunkDetail {
                ref milestone_slug, ..
            } => {
                if key.code == KeyCode::Char('/') {
                    self.slash_state = Some(SlashState::default());
                    return false;
                }
                if key.code == KeyCode::Esc {
                    let slug = milestone_slug.clone();
                    self.screen = Screen::MilestoneDetail { slug };
                } else if key.code == KeyCode::Char('q') {
                    return true;
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
                                        self.refresh_poll_targets();
                                        if let Ok(status) = cycle_status(&assay_dir) {
                                            self.cycle_slug = status.map(|cs| cs.milestone_slug);
                                        }
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

            Screen::AgentRun {
                ref mut scroll_offset,
                ..
            } => {
                match key.code {
                    KeyCode::Char('/') => {
                        self.slash_state = Some(SlashState::default());
                        return false;
                    }
                    KeyCode::Esc => {
                        self.screen = Screen::Dashboard;
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        *scroll_offset = scroll_offset.saturating_add(1);
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        *scroll_offset = scroll_offset.saturating_sub(1);
                    }
                    _ => {}
                }
                false
            }

            Screen::Settings { .. } => {
                if key.code == KeyCode::Char('/') {
                    self.slash_state = Some(SlashState::default());
                    return false;
                }
                // ── Model-section key handling ────────────────────────────
                // When model_focus is Some, intercept Tab/Esc/Char/Backspace
                // before falling through to provider-list navigation.
                let model_focus_val = if let Screen::Settings { model_focus, .. } = &self.screen {
                    *model_focus
                } else {
                    None
                };

                if let Some(focused) = model_focus_val {
                    match key.code {
                        KeyCode::Tab => {
                            // Cycle: 0→1→2→None (returns to provider list).
                            if let Screen::Settings {
                                ref mut model_focus,
                                ..
                            } = self.screen
                            {
                                *model_focus = if focused < 2 { Some(focused + 1) } else { None };
                            }
                            return false;
                        }
                        KeyCode::Esc => {
                            // Unfocus model section; stay on Settings.
                            if let Screen::Settings {
                                ref mut model_focus,
                                ..
                            } = self.screen
                            {
                                *model_focus = None;
                            }
                            return false;
                        }
                        KeyCode::Char('w') => {
                            // 'w' is the global save command — let it fall
                            // through to the provider-list handler below.
                        }
                        KeyCode::Char(c) => {
                            match focused {
                                0 => {
                                    if let Screen::Settings {
                                        ref mut planning_model,
                                        ..
                                    } = self.screen
                                    {
                                        planning_model.push(c);
                                    }
                                }
                                1 => {
                                    if let Screen::Settings {
                                        ref mut execution_model,
                                        ..
                                    } = self.screen
                                    {
                                        execution_model.push(c);
                                    }
                                }
                                _ => {
                                    if let Screen::Settings {
                                        ref mut review_model,
                                        ..
                                    } = self.screen
                                    {
                                        review_model.push(c);
                                    }
                                }
                            }
                            return false;
                        }
                        KeyCode::Backspace => {
                            match focused {
                                0 => {
                                    if let Screen::Settings {
                                        ref mut planning_model,
                                        ..
                                    } = self.screen
                                    {
                                        planning_model.pop();
                                    }
                                }
                                1 => {
                                    if let Screen::Settings {
                                        ref mut execution_model,
                                        ..
                                    } = self.screen
                                    {
                                        execution_model.pop();
                                    }
                                }
                                _ => {
                                    if let Screen::Settings {
                                        ref mut review_model,
                                        ..
                                    } = self.screen
                                    {
                                        review_model.pop();
                                    }
                                }
                            }
                            return false;
                        }
                        _ => {
                            return false;
                        }
                    }
                }

                // ── Provider-list key handling (model_focus is None) ──────
                match key.code {
                    KeyCode::Tab => {
                        // Enter model section, focus planning_model.
                        if let Screen::Settings {
                            ref mut model_focus,
                            ..
                        } = self.screen
                        {
                            *model_focus = Some(0);
                        }
                    }
                    KeyCode::Esc | KeyCode::Char('q') => {
                        self.screen = Screen::Dashboard;
                    }
                    KeyCode::Down => {
                        if let Screen::Settings { selected, .. } = &mut self.screen {
                            *selected = (*selected + 1) % 3;
                        }
                    }
                    KeyCode::Up => {
                        if let Screen::Settings { selected, .. } = &mut self.screen {
                            *selected = selected.checked_sub(1).unwrap_or(2);
                        }
                    }
                    KeyCode::Char('w') => {
                        // Extract what we need from the screen before taking
                        // a mutable borrow on self.project_root / self.config.
                        let (selected, pm_buf, em_buf, rm_buf) = if let Screen::Settings {
                            selected,
                            ref planning_model,
                            ref execution_model,
                            ref review_model,
                            ..
                        } = self.screen
                        {
                            (
                                selected,
                                planning_model.clone(),
                                execution_model.clone(),
                                review_model.clone(),
                            )
                        } else {
                            unreachable!("must be in Settings screen");
                        };
                        let kind = match selected {
                            1 => ProviderKind::OpenAi,
                            2 => ProviderKind::Ollama,
                            _ => ProviderKind::Anthropic,
                        };
                        match &self.project_root {
                            None => {
                                if let Screen::Settings { ref mut error, .. } = self.screen {
                                    *error = Some(
                                        "Cannot save: no project root. Run `assay init` first."
                                            .to_string(),
                                    );
                                }
                            }
                            Some(root) => {
                                // Require an existing config so we don't write a
                                // config.toml with an empty project_name (which would
                                // fail to reload on the next startup). Per D103.
                                let mut cfg = match self.config.clone() {
                                    Some(c) => c,
                                    None => {
                                        if let Screen::Settings { ref mut error, .. } = self.screen
                                        {
                                            *error = Some("Cannot save: no project config found. Run `assay init` first.".to_string());
                                        }
                                        return false;
                                    }
                                };
                                cfg.provider = Some(ProviderConfig {
                                    provider: kind,
                                    // Use in-screen buffers; empty → None.
                                    planning_model: Some(pm_buf).filter(|s| !s.is_empty()),
                                    execution_model: Some(em_buf).filter(|s| !s.is_empty()),
                                    review_model: Some(rm_buf).filter(|s| !s.is_empty()),
                                });
                                match config_save(root, &cfg) {
                                    Ok(()) => {
                                        self.config = Some(cfg);
                                        // Refresh cycle_slug after settings save.
                                        // On error, preserve existing slug rather than
                                        // silently clearing it.
                                        let assay_dir = root.join(".assay");
                                        if let Ok(status) = cycle_status(&assay_dir) {
                                            self.cycle_slug = status.map(|cs| cs.milestone_slug);
                                        }
                                        self.screen = Screen::Dashboard;
                                    }
                                    Err(e) => {
                                        if let Screen::Settings { ref mut error, .. } = self.screen
                                        {
                                            *error = Some(format!(
                                                "Save failed: {e}. Check that \
                                                 .assay/config.toml is writable."
                                            ));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
                false
            }

            Screen::Analytics => {
                match key.code {
                    KeyCode::Esc => {
                        self.screen = Screen::Dashboard;
                    }
                    KeyCode::Char('q') => return true,
                    _ => {}
                }
                false
            }

            Screen::McpPanel { .. } => self.handle_mcp_panel_event(key),
        }
    }
}

// ── Screen renderers ──────────────────────────────────────────────────────────

/// Render the live agent run screen.
///
/// Layout: bordered block with title at top; inside → scrollable line list
/// (fills available height) + 1-row status line at the bottom.
fn draw_agent_run(
    frame: &mut ratatui::Frame,
    area: Rect,
    chunk_slug: &str,
    lines: &[String],
    scroll_offset: usize,
    status: &AgentRunStatus,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Agent Run: {chunk_slug} "));
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let [content_area, status_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(inner);

    // Line list.
    let visible_rows = content_area.height as usize;
    if lines.is_empty() {
        let placeholder = List::new(vec![
            ListItem::new("Starting…").style(Style::default().dim()),
        ]);
        frame.render_widget(placeholder, content_area);
    } else {
        let safe_start = scroll_offset.min(lines.len().saturating_sub(1));
        let safe_end = (safe_start + visible_rows).min(lines.len());
        let items: Vec<ListItem> = lines[safe_start..safe_end]
            .iter()
            .map(|l| ListItem::new(l.as_str()))
            .collect();
        let list = List::new(items);
        frame.render_widget(list, content_area);
    }

    // Status line.
    let (status_text, status_style) = match status {
        AgentRunStatus::Running => (
            "● Running…  Esc: back".to_string(),
            Style::default().fg(Color::Yellow),
        ),
        AgentRunStatus::Done { exit_code } => (
            format!("✓ Done (exit {exit_code})  Esc: back"),
            Style::default().fg(Color::Green),
        ),
        AgentRunStatus::Failed { exit_code } => (
            format!("✗ Failed (exit {exit_code})  Esc: back"),
            Style::default().fg(Color::Red),
        ),
    };
    let status_line = Paragraph::new(Line::from(Span::styled(status_text, status_style)));
    frame.render_widget(status_line, status_area);
}

/// Render a load-error screen when milestone data could not be read.
fn draw_load_error(frame: &mut ratatui::Frame, area: Rect, msg: &str) {
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
fn draw_no_project(frame: &mut ratatui::Frame, area: Rect) {
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
///
/// `pr_statuses` provides cached PR badge data per milestone slug (D097 —
/// pass individual fields, not `&mut App`).
fn draw_dashboard(
    frame: &mut ratatui::Frame,
    area: Rect,
    milestones: &[Milestone],
    list_state: &mut ListState,
    pr_statuses: &HashMap<String, PrStatusInfo>,
) {
    use assay_core::pr::PrStatusState;

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
                let status_label = match m.status {
                    MilestoneStatus::Draft => "Draft",
                    MilestoneStatus::InProgress => "In Progress",
                    MilestoneStatus::Verify => "Verify",
                    MilestoneStatus::Complete => "✓ Done",
                };

                let mut spans: Vec<Span> =
                    vec![Span::raw(format!("  {status_label:<12}  {}", m.name))];

                // PR status badge (if polled).
                if let Some(info) = pr_statuses.get(&m.slug) {
                    let (state_icon, state_color) = match info.state {
                        PrStatusState::Open => ("🟢 OPEN", Color::Green),
                        PrStatusState::Merged => ("🟣 MERGED", Color::Magenta),
                        PrStatusState::Closed => ("🔴 CLOSED", Color::Red),
                    };
                    spans.push(Span::raw("  "));
                    spans.push(Span::styled(state_icon, Style::default().fg(state_color)));

                    // CI summary.
                    let total = info.ci_pass + info.ci_fail + info.ci_pending;
                    if total > 0 {
                        let ci_text = if info.ci_fail > 0 {
                            format!(" ✗{} fail", info.ci_fail)
                        } else {
                            format!(" ✓{}/{}", info.ci_pass, total)
                        };
                        let ci_color = if info.ci_fail > 0 {
                            Color::Red
                        } else if info.ci_pending > 0 {
                            Color::Yellow
                        } else {
                            Color::Green
                        };
                        spans.push(Span::styled(ci_text, Style::default().fg(ci_color)));
                    }

                    // Review decision.
                    if !info.review_decision.is_empty() {
                        let abbrev = match info.review_decision.as_str() {
                            "APPROVED" => "✓rvw",
                            "CHANGES_REQUESTED" => "△rvw",
                            "REVIEW_REQUIRED" => "?rvw",
                            other => other,
                        };
                        spans.push(Span::styled(format!(" {abbrev}"), Style::default().dim()));
                    }
                }

                ListItem::new(Line::from(spans))
            })
            .collect();
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL))
            .highlight_style(Style::default().bold().reversed());
        frame.render_stateful_widget(list, list_area, list_state);
    }

    // Hint bar.
    let hint = Paragraph::new(Line::from("↑↓ navigate · Enter open · n new · q quit").dim());
    frame.render_widget(hint, hint_area);
}

/// Render the chunk list for a single milestone.
fn draw_milestone_detail(
    frame: &mut ratatui::Frame,
    area: Rect,
    milestone: Option<&Milestone>,
    list_state: &mut ListState,
) {
    let [title_area, list_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // Title bar.
    let title_text = milestone
        .map(|ms| format!(" Milestones › {} ", ms.name))
        .unwrap_or_else(|| " Loading… ".to_string());
    let title = Paragraph::new(Line::from(title_text).bold());
    frame.render_widget(title, title_area);

    // Chunk list.
    if let Some(ms) = milestone {
        if ms.chunks.is_empty() {
            let msg = Paragraph::new(Line::from("No chunks in this milestone.").dim())
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(msg, list_area);
        } else {
            let mut sorted_chunks = ms.chunks.clone();
            sorted_chunks.sort_by_key(|c| c.order);
            let items: Vec<ListItem> = sorted_chunks
                .iter()
                .map(|chunk| {
                    let icon = if ms.completed_chunks.contains(&chunk.slug) {
                        "✓"
                    } else {
                        "·"
                    };
                    ListItem::new(Line::from(format!("  {icon}  {}", chunk.slug)))
                })
                .collect();
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL))
                .highlight_style(Style::default().bold().reversed());
            frame.render_stateful_widget(list, list_area, list_state);
        }
    } else {
        let msg = Paragraph::new(Line::from("Loading…").dim())
            .block(Block::default().borders(Borders::ALL));
        frame.render_widget(msg, list_area);
    }

    // Hint bar.
    let hint = Paragraph::new(Line::from("↑↓ navigate · Enter open chunk · Esc back").dim());
    frame.render_widget(hint, hint_area);
}

/// Join criteria from a spec with results from the latest gate run.
///
/// For each criterion: look it up by name in `run.summary.results`.
/// - Not found → `(criterion, None)` (Pending)
/// - Found with `result = Some(gate_result)` → `(criterion, Some(gate_result.passed))`
/// - Found with `result = None` (skipped) → `(criterion, None)` (Pending/skipped)
fn join_results<'a>(
    criteria: &'a [Criterion],
    run: Option<&'a GateRunRecord>,
) -> Vec<(&'a Criterion, Option<bool>)> {
    criteria
        .iter()
        .map(|criterion| {
            let result = run
                .and_then(|r| {
                    r.summary
                        .results
                        .iter()
                        .find(|cr| cr.criterion_name == criterion.name)
                })
                .and_then(|cr| cr.result.as_ref())
                .map(|gate_result| gate_result.passed);
            (criterion, result)
        })
        .collect()
}

/// Render the chunk detail screen with a table of criteria and their results.
fn draw_chunk_detail(
    frame: &mut ratatui::Frame,
    area: Rect,
    chunk_slug: &str,
    spec: Option<&GatesSpec>,
    spec_note: Option<&str>,
    run: Option<&GateRunRecord>,
) {
    let [title_area, table_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(area);

    // Title bar.
    let title = Paragraph::new(Line::from(format!("  {chunk_slug}  — Criteria")).bold());
    frame.render_widget(title, title_area);

    // Table or message.
    if let Some(gs) = spec {
        let joined = join_results(&gs.criteria, run);
        let rows: Vec<Row> = joined
            .iter()
            .map(|(criterion, result_opt)| {
                let (icon, icon_style) = match result_opt {
                    Some(true) => ("✓", Style::default().fg(Color::Green)),
                    Some(false) => ("✗", Style::default().fg(Color::Red)),
                    None => ("?", Style::default().dim()),
                };
                Row::new(vec![
                    Cell::from(icon).style(icon_style),
                    Cell::from(criterion.name.as_str()),
                    Cell::from(criterion.description.as_str()),
                ])
            })
            .collect();

        let widths = [
            Constraint::Length(3),
            Constraint::Length(24),
            Constraint::Fill(1),
        ];
        let table = Table::new(rows, widths).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" {} criteria ", gs.name)),
        );
        frame.render_widget(table, table_area);
    } else {
        let msg = spec_note.unwrap_or("No spec data");
        let paragraph =
            Paragraph::new(Line::from(msg).dim()).block(Block::default().borders(Borders::ALL));
        frame.render_widget(paragraph, table_area);
    }

    // Hint bar.
    let hint = Paragraph::new(Line::from("Esc back").dim());
    frame.render_widget(hint, hint_area);
}

/// Render the provider configuration screen.
///
/// Full-screen bordered block listing three provider options (Anthropic,
/// OpenAI, Ollama) with the currently selected one highlighted and a brief
/// legend of key hints at the bottom.
#[allow(clippy::too_many_arguments)]
fn draw_settings(
    frame: &mut ratatui::Frame,
    area: Rect,
    config: Option<&assay_types::Config>,
    selected: usize,
    error: Option<&str>,
    model_focus: Option<usize>,
    planning_model: &str,
    execution_model: &str,
    review_model: &str,
) {
    let block = Block::default()
        .title(" Provider Configuration ")
        .borders(Borders::ALL);
    frame.render_widget(block.clone(), area);

    let inner = block.inner(area);

    // Layout: provider list (3 rows) + model section (4 rows) + hints at bottom.
    let [provider_area, model_area, hint_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(4),
        Constraint::Length(if error.is_some() { 3 } else { 2 }),
    ])
    .areas(inner);

    // ── Provider list ────────────────────────────────────────────────────────
    let providers = ["Anthropic (Claude)", "OpenAI (GPT)", "Ollama (local)"];
    let current_kind = config
        .and_then(|c| c.provider.as_ref())
        .map(|p| p.provider)
        .unwrap_or(ProviderKind::Anthropic);
    let saved_idx = match current_kind {
        ProviderKind::Anthropic => 0,
        ProviderKind::OpenAi => 1,
        ProviderKind::Ollama => 2,
    };

    let items: Vec<ListItem> = providers
        .iter()
        .enumerate()
        .map(|(i, name)| {
            let prefix = if i == selected && model_focus.is_none() {
                "▶ "
            } else {
                "  "
            };
            let suffix = if i == saved_idx { "  [saved]" } else { "" };
            let label = format!("{prefix}{name}{suffix}");
            let style = if i == selected && model_focus.is_none() {
                Style::default().bold().fg(Color::Cyan)
            } else {
                Style::default()
            };
            ListItem::new(label).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, provider_area);

    // ── Model fields ─────────────────────────────────────────────────────────
    let model_labels = ["Planning model", "Execution model", "Review model"];
    let model_values = [planning_model, execution_model, review_model];
    let mut model_lines: Vec<Line> = vec![Line::from(Span::styled(
        "  Models:",
        Style::default().dim(),
    ))];
    for (i, (label, value)) in model_labels.iter().zip(model_values.iter()).enumerate() {
        let text = format!("  {label:<18} [{value}]");
        let style = if model_focus == Some(i) {
            Style::default().bold().fg(Color::Cyan)
        } else {
            Style::default().dim()
        };
        model_lines.push(Line::from(Span::styled(text, style)));
    }
    let model_para = Paragraph::new(model_lines);
    frame.render_widget(model_para, model_area);

    // ── Key hints (and optional error above them) ─────────────────────────────
    let mut hint_lines: Vec<Line> = Vec::new();
    if let Some(msg) = error {
        hint_lines.push(Line::from(Span::styled(
            format!("Error: {msg}"),
            Style::default().fg(Color::Red),
        )));
    }
    let hint_text = if model_focus.is_some() {
        "Tab next field  Esc exit models  w save"
    } else {
        "↑↓ select  Tab edit models  w save  Esc cancel"
    };
    hint_lines.push(Line::from(Span::styled(hint_text, Style::default().dim())));

    let hint = Paragraph::new(hint_lines);
    frame.render_widget(hint, hint_area);
}

/// Render the analytics screen with failure frequency and milestone velocity tables.
///
/// If `report` is `None` or both result sets are empty, renders a centered
/// "No analytics data available" message instead of the two-table layout.
fn draw_analytics(frame: &mut ratatui::Frame, area: Rect, report: Option<&AnalyticsReport>) {
    let report = match report {
        Some(r) if !r.failure_frequency.is_empty() || !r.milestone_velocity.is_empty() => r,
        _ => {
            let msg = Paragraph::new(
                Line::from("No analytics data available")
                    .centered()
                    .style(Style::default().dim()),
            )
            .block(Block::default().borders(Borders::ALL).title(" Analytics "));
            frame.render_widget(msg, area);
            return;
        }
    };

    // Layout: title, failure frequency table, velocity table, hint line.
    // Velocity table height = entries + 3 (header + top/bottom border), capped at 12 rows.
    let [title_area, freq_area, vel_area, hint_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(
            (report.milestone_velocity.len() as u16)
                .saturating_add(3)
                .min(12),
        ),
        Constraint::Length(1),
    ])
    .areas(area);

    let title = Paragraph::new(Line::from(" Analytics ").bold());
    frame.render_widget(title, title_area);

    // ── Failure Frequency Table ──────────────────────────────────────────
    if report.failure_frequency.is_empty() {
        let msg = Paragraph::new(Line::from("No gate run history found.").dim()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Gate Failure Frequency "),
        );
        frame.render_widget(msg, freq_area);
    } else {
        let freq_rows: Vec<Row> = report
            .failure_frequency
            .iter()
            .map(|f| {
                let rate = if f.total_runs > 0 {
                    f.fail_count as f64 / f.total_runs as f64 * 100.0
                } else {
                    0.0
                };
                let rate_str = format!("{rate:.1}%");
                let rate_color = if rate > 50.0 {
                    Color::Red
                } else if rate > 0.0 {
                    Color::Yellow
                } else {
                    Color::Green
                };
                let enforcement_label = match f.enforcement {
                    Enforcement::Required => "Required",
                    Enforcement::Advisory => "Advisory",
                };
                Row::new(vec![
                    Cell::from(f.spec_name.as_str()),
                    Cell::from(f.criterion_name.as_str()),
                    Cell::from(f.fail_count.to_string()),
                    Cell::from(f.total_runs.to_string()),
                    Cell::from(rate_str).fg(rate_color),
                    Cell::from(enforcement_label),
                ])
            })
            .collect();

        let freq_widths = [
            Constraint::Length(20),
            Constraint::Fill(1),
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Length(10),
        ];
        let freq_table = Table::new(freq_rows, freq_widths)
            .header(
                Row::new(vec![
                    "Spec",
                    "Criterion",
                    "Fails",
                    "Runs",
                    "Rate",
                    "Enforce",
                ])
                .bold(),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Gate Failure Frequency "),
            );
        frame.render_widget(freq_table, freq_area);
    }

    // ── Milestone Velocity Table ─────────────────────────────────────────
    if report.milestone_velocity.is_empty() {
        let msg = Paragraph::new(Line::from("No milestones with completed chunks.").dim()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Milestone Velocity "),
        );
        frame.render_widget(msg, vel_area);
    } else {
        let vel_rows: Vec<Row> = report
            .milestone_velocity
            .iter()
            .map(|v| {
                Row::new(vec![
                    Cell::from(v.milestone_name.as_str()),
                    Cell::from(format!("{}/{}", v.chunks_completed, v.total_chunks)),
                    Cell::from(format!("{:.1}", v.days_elapsed)),
                    Cell::from(format!("{:.1}/day", v.chunks_per_day)),
                ])
            })
            .collect();

        let vel_widths = [
            Constraint::Fill(1),
            Constraint::Length(10),
            Constraint::Length(8),
            Constraint::Length(10),
        ];
        let vel_table = Table::new(vel_rows, vel_widths)
            .header(Row::new(vec!["Milestone", "Chunks", "Days", "Velocity"]).bold())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Milestone Velocity "),
            );
        frame.render_widget(vel_table, vel_area);
    }

    let hint = Paragraph::new(Line::from("Esc back  q quit").dim());
    frame.render_widget(hint, hint_area);
}

/// Render the persistent one-line status bar showing project context.
///
/// Shows: `<project_name>  ·  <cycle_slug>  ·  ? help  q quit` (dim hints).
/// When `project_name` is empty and `cycle_slug` is `None`, only the key hints
/// are shown so the bar is never blank.
fn draw_status_bar(
    frame: &mut ratatui::Frame,
    area: Rect,
    project_name: &str,
    cycle_slug: Option<&str>,
) {
    let sep = Span::styled("  ·  ", Style::default().dim());
    let mut spans: Vec<Span> = Vec::new();

    if !project_name.is_empty() {
        spans.push(Span::raw(project_name.to_string()));
        spans.push(sep.clone());
    }

    let slug_text = cycle_slug.unwrap_or("");
    spans.push(Span::styled(slug_text.to_string(), Style::default().dim()));

    spans.push(sep);
    spans.push(Span::styled("? help  q quit", Style::default().dim()));

    let bar = Paragraph::new(Line::from(spans));
    frame.render_widget(bar, area);
}

/// Render a centered bordered help overlay listing all keybindings.
///
/// Renders on top of all other content. The caller is responsible for calling
/// this only when `App::show_help` is `true`. Uses `Clear` to erase background
/// content beneath the popup.
fn draw_help_overlay(frame: &mut ratatui::Frame, area: Rect) {
    let w = area.width.min(62);
    let h = 23;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    let popup = Rect::new(x, y, w, h);

    frame.render_widget(Clear, popup);

    let block = Block::bordered().title(" Keybindings — press ? or Esc to close ");
    frame.render_widget(block.clone(), popup);
    let inner = block.inner(popup);

    let rows = vec![
        Row::new(vec![
            Cell::from("Global").style(Style::default().bold()),
            Cell::from(""),
        ]),
        Row::new(vec![
            Cell::from("  ?"),
            Cell::from("Toggle this help overlay"),
        ]),
        Row::new(vec![Cell::from("  q"), Cell::from("Quit")]),
        Row::new(vec![
            Cell::from("Dashboard").style(Style::default().bold()),
            Cell::from(""),
        ]),
        Row::new(vec![
            Cell::from("  ↑↓"),
            Cell::from("Navigate milestone list"),
        ]),
        Row::new(vec![Cell::from("  Enter"), Cell::from("Open milestone")]),
        Row::new(vec![
            Cell::from("  n"),
            Cell::from("New milestone (wizard)"),
        ]),
        Row::new(vec![Cell::from("  a"), Cell::from("Analytics")]),
        Row::new(vec![Cell::from("  s"), Cell::from("Settings")]),
        Row::new(vec![
            Cell::from("Detail views").style(Style::default().bold()),
            Cell::from(""),
        ]),
        Row::new(vec![Cell::from("  ↑↓"), Cell::from("Navigate chunk list")]),
        Row::new(vec![Cell::from("  Enter"), Cell::from("Open chunk")]),
        Row::new(vec![Cell::from("  Esc"), Cell::from("Back to parent")]),
        Row::new(vec![
            Cell::from("Wizard").style(Style::default().bold()),
            Cell::from(""),
        ]),
        Row::new(vec![
            Cell::from("  Enter"),
            Cell::from("Next step / confirm"),
        ]),
        Row::new(vec![
            Cell::from("  Backspace"),
            Cell::from("Delete / previous step"),
        ]),
        Row::new(vec![Cell::from("  Esc"), Cell::from("Cancel wizard")]),
        Row::new(vec![
            Cell::from("Settings").style(Style::default().bold()),
            Cell::from(""),
        ]),
        Row::new(vec![Cell::from("  ↑↓"), Cell::from("Select provider")]),
        Row::new(vec![Cell::from("  w"), Cell::from("Save settings")]),
        Row::new(vec![Cell::from("  Esc / q"), Cell::from("Cancel")]),
    ];

    let widths = [Constraint::Length(14), Constraint::Fill(1)];
    let table = Table::new(rows, widths);
    frame.render_widget(table, inner);
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
