//! Application state, screen dispatch, and renderers for assay-tui.
//!
//! `App` owns all mutable state. `Screen` drives the render/event dispatch.
//! Both are public so integration tests can construct `App` directly and
//! assert on `app.screen` after driving key events.

use std::path::PathBuf;
use std::sync::mpsc;

use assay_core::config::save as config_save;
use assay_core::history;
use assay_core::milestone::{cycle_status, milestone_load, milestone_scan};
use assay_core::spec::{SpecEntry, load_spec_entry_with_diagnostics};
use assay_core::wizard::create_from_inputs;
use assay_types::{
    Criterion, GateRunRecord, GatesSpec, Milestone, MilestoneStatus, ProviderConfig, ProviderKind,
};
use crossterm::event::KeyCode;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row, Table,
};

use crate::event::TuiEvent;
use crate::wizard::{WizardAction, WizardState, draw_wizard, handle_wizard_event};

// ── AgentRun status ───────────────────────────────────────────────────────────

/// Status of a running or completed agent session shown on `Screen::AgentRun`.
#[derive(Debug, PartialEq, Eq)]
pub enum AgentRunStatus {
    /// Agent subprocess is still running.
    Running,
    /// Agent subprocess exited cleanly.
    Done { exit_code: i32 },
    /// Agent subprocess exited with a non-zero code.
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
    },
    /// Streaming agent run output for a chunk.
    AgentRun {
        /// Slug of the chunk being run.
        chunk_slug: String,
        /// Lines streamed from the agent subprocess so far.
        lines: Vec<String>,
        /// Current scroll offset (number of lines scrolled down).
        scroll_offset: usize,
        /// Current status of the agent subprocess.
        status: AgentRunStatus,
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
    /// Background thread running the agent subprocess.
    /// Holds a `JoinHandle<i32>` that resolves to the exit code when joined.
    pub agent_thread: Option<std::thread::JoinHandle<i32>>,
    /// Sender half of the TUI event channel.
    ///
    /// Set from `run()` after the channel is created. The `r` key handler uses
    /// this to forward agent lines and done signals into the event loop.
    pub event_tx: Option<mpsc::Sender<TuiEvent>>,
    /// Whether the help overlay is currently visible.
    pub show_help: bool,
    /// Slug of the currently active (InProgress) milestone, or `None` when no
    /// milestone is in progress.  Used by the status bar renderer.
    pub cycle_slug: Option<String>,
    /// Loaded project config (used by status bar and settings screen).
    pub config: Option<assay_types::Config>,
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
                    let slug = cycle_status(&assay_dir)
                        .ok()
                        .flatten()
                        .map(|cs| cs.milestone_slug);
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

        let config = project_root
            .as_deref()
            .and_then(|root| assay_core::config::load(root).ok());

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
            agent_thread: None,
            event_tx: None,
            show_help: false,
            cycle_slug,
            config,
        })
    }

    /// Draw the current screen into `frame`.
    pub fn draw(&mut self, frame: &mut ratatui::Frame) {
        let [content_area, status_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

        match &self.screen {
            Screen::NoProject => draw_no_project(frame, content_area),
            Screen::Dashboard => {
                draw_dashboard(frame, content_area, &self.milestones, &mut self.list_state)
            }
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
            Screen::Settings { selected, error } => {
                draw_settings(
                    frame,
                    content_area,
                    self.config.as_ref(),
                    *selected,
                    error.as_deref(),
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
                        self.handle_r_key();
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
                        self.screen = Screen::Settings {
                            selected,
                            error: None,
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
                if key.code == KeyCode::Esc {
                    let slug = milestone_slug.clone();
                    self.screen = Screen::MilestoneDetail { slug };
                } else if key.code == KeyCode::Char('q') {
                    return true;
                }
                false
            }

            Screen::AgentRun { .. } => {
                match key.code {
                    KeyCode::Esc => {
                        self.screen = Screen::Dashboard;
                    }
                    KeyCode::Char('q') => return true,
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
                                        self.cycle_slug = cycle_status(&assay_dir)
                                            .ok()
                                            .flatten()
                                            .map(|cs| cs.milestone_slug);
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

            Screen::Settings { .. } => {
                match key.code {
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
                        // Save provider selection to config.toml.
                        let (selected, _error_slot) = if let Screen::Settings {
                            selected,
                            ref mut error,
                        } = self.screen
                        {
                            (selected, error)
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
                                // Build or mutate config.
                                let mut cfg =
                                    self.config.clone().unwrap_or_else(|| assay_types::Config {
                                        project_name: String::new(),
                                        specs_dir: "specs/".to_string(),
                                        gates: None,
                                        guard: None,
                                        worktree: None,
                                        sessions: None,
                                        provider: None,
                                    });
                                cfg.provider = Some(ProviderConfig {
                                    provider: kind,
                                    planning_model: cfg
                                        .provider
                                        .as_ref()
                                        .and_then(|p| p.planning_model.clone()),
                                    execution_model: cfg
                                        .provider
                                        .as_ref()
                                        .and_then(|p| p.execution_model.clone()),
                                    review_model: cfg
                                        .provider
                                        .as_ref()
                                        .and_then(|p| p.review_model.clone()),
                                });
                                match config_save(root, &cfg) {
                                    Ok(()) => {
                                        self.config = Some(cfg);
                                        // Refresh cycle_slug after settings save.
                                        let assay_dir = root.join(".assay");
                                        self.cycle_slug = cycle_status(&assay_dir)
                                            .ok()
                                            .flatten()
                                            .map(|cs| cs.milestone_slug);
                                        self.screen = Screen::Dashboard;
                                    }
                                    Err(e) => {
                                        if let Screen::Settings { ref mut error, .. } = self.screen
                                        {
                                            *error = Some(format!("Save failed: {e}"));
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
        }
    }

    /// Handle the `r` key press from the Dashboard screen.
    ///
    /// Spawns the agent for the active chunk if:
    /// - `self.project_root` is `Some`
    /// - `cycle_status()` returns an `active_chunk_slug`
    /// - `self.event_tx` is `Some` (i.e. we're inside a real `run()` loop)
    ///
    /// Uses the two-channel design: `line_tx/line_rx` for streamed lines,
    /// `exit_tx/exit_rx` for the exit code. The bridge thread drains `line_rx`,
    /// then receives the exit code from `exit_rx`, then sends `TuiEvent::AgentDone`.
    /// This ensures the `JoinHandle<i32>` never needs to be joined from `App`.
    fn handle_r_key(&mut self) {
        let root = match &self.project_root {
            Some(r) => r.clone(),
            None => return,
        };
        let event_tx = match &self.event_tx {
            Some(tx) => tx.clone(),
            None => return,
        };

        let assay_dir = root.join(".assay");
        let status = match assay_core::milestone::cycle_status(&assay_dir) {
            Ok(Some(s)) => s,
            Ok(None) => return, // no active chunk — valid no-op
            Err(e) => {
                // Surface the I/O error in an AgentRun screen so the user knows why r did nothing.
                self.screen = Screen::AgentRun {
                    chunk_slug: String::new(),
                    lines: vec![format!("[error] failed to read cycle status: {e}")],
                    scroll_offset: 0,
                    status: AgentRunStatus::Failed { exit_code: -1 },
                };
                return;
            }
        };
        let chunk_slug = match status.active_chunk_slug {
            Some(slug) => slug,
            None => return,
        };

        // Build a minimal ManifestSession referencing the active chunk.
        let session = assay_types::ManifestSession {
            spec: chunk_slug.clone(),
            name: None,
            settings: None,
            hooks: vec![],
            prompt_layers: vec![],
            file_scope: vec![],
            shared_files: vec![],
            depends_on: vec![],
        };
        let profile = assay_core::pipeline::build_harness_profile(&session);
        let claude_config = assay_harness::claude::generate_config(&profile);

        // Create a temp dir for harness config files.
        let tmp = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => {
                self.screen = Screen::AgentRun {
                    chunk_slug,
                    lines: vec![format!("[error] cannot create temp directory: {e}")],
                    scroll_offset: 0,
                    status: AgentRunStatus::Failed { exit_code: -1 },
                };
                return;
            }
        };

        // Write harness config; abort if it fails (disk full, permissions, etc.).
        if let Err(e) = assay_harness::claude::write_config(&claude_config, tmp.path()) {
            self.screen = Screen::AgentRun {
                chunk_slug,
                lines: vec![format!("[error] failed to write harness config: {e}")],
                scroll_offset: 0,
                status: AgentRunStatus::Failed { exit_code: -1 },
            };
            return;
        }

        let cli_args = assay_harness::claude::build_cli_args(&claude_config);

        // Two channels: one for streamed lines, one for the exit code.
        let (line_tx, line_rx) = mpsc::channel::<String>();
        let (exit_tx, exit_rx) = mpsc::channel::<i32>();

        // Spawn the agent subprocess; propagate spawn errors to the AgentRun screen.
        let handle = match assay_core::pipeline::launch_agent_streaming(&cli_args, &root, line_tx) {
            Ok(h) => h,
            Err(e) => {
                self.screen = Screen::AgentRun {
                    chunk_slug,
                    lines: vec![format!("[error] failed to spawn agent: {e}")],
                    scroll_offset: 0,
                    status: AgentRunStatus::Failed { exit_code: -1 },
                };
                return;
            }
        };

        // Spawn a second thread that joins the handle and sends the exit code.
        std::thread::spawn(move || {
            let exit_code = handle.join().unwrap_or(-1);
            let _ = exit_tx.send(exit_code);
        });

        // Bridge thread: forward lines → TuiEvent::AgentLine, then exit code → AgentDone.
        std::thread::spawn(move || {
            for line in line_rx {
                let _ = event_tx.send(TuiEvent::AgentLine(line));
            }
            // line_rx disconnected = agent stdout closed. Now wait for exit code.
            let exit_code = exit_rx.recv().unwrap_or(-1);
            let _ = event_tx.send(TuiEvent::AgentDone { exit_code });
        });

        // Keep tmp dir alive for the duration of the subprocess run.
        std::mem::forget(tmp);

        // App no longer owns the JoinHandle — it moved to the exit-code thread.
        self.agent_thread = None;

        self.screen = Screen::AgentRun {
            chunk_slug,
            lines: vec![],
            scroll_offset: 0,
            status: AgentRunStatus::Running,
        };
    }

    /// Append a line from the streaming agent output to the current `AgentRun` screen.
    ///
    /// No-op when the current screen is not `Screen::AgentRun`.
    pub fn handle_agent_line(&mut self, line: String) {
        if let Screen::AgentRun {
            ref mut lines,
            ref mut scroll_offset,
            ..
        } = self.screen
        {
            lines.push(line);
            // Auto-scroll: keep the last lines visible (assume ~20 visible rows).
            const VISIBLE_HEIGHT: usize = 20;
            *scroll_offset = lines.len().saturating_sub(VISIBLE_HEIGHT);
        }
    }

    /// Transition the `AgentRun` screen to `Done` or `Failed` based on the exit code.
    ///
    /// Refreshes milestone data and `cycle_slug` immediately after the agent exits
    /// so gate results are up to date.
    ///
    /// With the revised two-channel design from T04, `agent_thread` is `None` when
    /// `r` is pressed (the handle is moved into the exit-code thread). If for some
    /// reason a handle is present (e.g. set externally in tests), it is joined here
    /// as a defensive cleanup measure.
    ///
    /// Honors D098: borrows from `self.project_root` are completed before
    /// `self.screen` is mutated.
    pub fn handle_agent_done(&mut self, exit_code: i32) {
        // Defensive join in case agent_thread was set externally (e.g. tests).
        // In production use via `r` key, this is always None.
        if let Some(handle) = self.agent_thread.take()
            && let Err(e) = handle.join()
        {
            eprintln!("agent_thread join error: {e:?}");
        }

        // Refresh milestone data — borrow project_root first, mutate screen after (D098).
        if let Some(ref root) = self.project_root {
            let assay_dir = root.join(".assay");
            self.milestones = milestone_scan(&assay_dir).unwrap_or_default();
            self.cycle_slug = cycle_status(&assay_dir)
                .ok()
                .flatten()
                .map(|s| s.milestone_slug);
        }

        // Update AgentRun status.
        let new_status = if exit_code == 0 {
            AgentRunStatus::Done { exit_code }
        } else {
            AgentRunStatus::Failed { exit_code }
        };
        if let Screen::AgentRun { ref mut status, .. } = self.screen {
            *status = new_status;
        }
    }
}

// ── Screen renderers ──────────────────────────────────────────────────────────

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
fn draw_dashboard(
    frame: &mut ratatui::Frame,
    area: Rect,
    milestones: &[Milestone],
    list_state: &mut ListState,
) {
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

/// Render the persistent one-line status bar showing project context.
/// Render the provider configuration screen.
///
/// Full-screen bordered block listing three provider options (Anthropic,
/// OpenAI, Ollama) with the currently selected one highlighted and a brief
/// legend of key hints at the bottom.
fn draw_settings(
    frame: &mut ratatui::Frame,
    area: Rect,
    config: Option<&assay_types::Config>,
    selected: usize,
    error: Option<&str>,
) {
    let block = Block::default()
        .title(" Provider Configuration ")
        .borders(Borders::ALL);
    frame.render_widget(block.clone(), area);

    let inner = block.inner(area);

    // Layout: list of providers + optional error + key hints at bottom.
    let [list_area, hint_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(if error.is_some() { 3 } else { 2 }),
    ])
    .areas(inner);

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
            let prefix = if i == selected { "▶ " } else { "  " };
            let suffix = if i == saved_idx { "  [saved]" } else { "" };
            let label = format!("{prefix}{name}{suffix}");
            let style = if i == selected {
                Style::default().bold().fg(Color::Cyan)
            } else {
                Style::default()
            };
            ListItem::new(label).style(style)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, list_area);

    // Key hints (and optional error above them).
    let mut hint_lines: Vec<Line> = Vec::new();
    if let Some(msg) = error {
        hint_lines.push(Line::from(Span::styled(
            format!("Error: {msg}"),
            Style::default().fg(Color::Red),
        )));
    }
    hint_lines.push(Line::from(Span::styled(
        "↑↓ select  w save  Esc cancel",
        Style::default().dim(),
    )));

    let hint = Paragraph::new(hint_lines);
    frame.render_widget(hint, hint_area);
}

/// Render the streaming agent run output screen.
///
/// Layout:
/// - Outer bordered block titled "Agent Run: {chunk_slug}"
/// - Content area: scrollable list of output lines from the agent
/// - Status line (1 row): "Running…" (yellow) / "Done (exit 0)" (green) /
///   "Failed (exit N)" (red)
///
/// Accepts individual field params (D097) rather than `&mut App`.
fn draw_agent_run(
    frame: &mut ratatui::Frame,
    area: Rect,
    chunk_slug: &str,
    lines: &[String],
    scroll_offset: usize,
    status: &AgentRunStatus,
) {
    let block = Block::default()
        .title(format!(" Agent Run: {chunk_slug} "))
        .borders(Borders::ALL);
    frame.render_widget(block.clone(), area);
    let inner = block.inner(area);

    // Inner layout: content area + 1-row status line.
    let [content_area, status_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(inner);

    // Content area: list of agent output lines with scroll offset.
    let items: Vec<ListItem> = lines
        .iter()
        .skip(scroll_offset)
        .map(|l| ListItem::new(l.as_str()))
        .collect();
    let list = List::new(items);
    frame.render_widget(list, content_area);

    // Status line.
    let (status_text, status_style) = match status {
        AgentRunStatus::Running => ("Running…".to_string(), Style::default().fg(Color::Yellow)),
        AgentRunStatus::Done { exit_code } => (
            format!("Done (exit {exit_code})"),
            Style::default().fg(Color::Green),
        ),
        AgentRunStatus::Failed { exit_code } => (
            format!("Failed (exit {exit_code})"),
            Style::default().fg(Color::Red),
        ),
    };
    let status_line = Paragraph::new(Line::from(Span::styled(status_text, status_style)));
    frame.render_widget(status_line, status_area);
}

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
    let h = 22;
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
