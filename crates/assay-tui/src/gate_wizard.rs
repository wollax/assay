//! Gate wizard: state machine, event handler, and renderer.
//!
//! This module provides the multi-step form that collects gate spec fields
//! (name, description, extends, includes, criteria, preconditions) and on
//! completion returns `GateWizardAction::Submit(GateWizardInput)` so the
//! caller can invoke `assay_core::wizard::apply_gate_wizard`.
//!
//! No validation logic lives here (WIZT-02). The only UX guard is the
//! empty-name check at step 0 — the real validation is owned by `apply_gate_wizard`.

use std::collections::HashSet;

use assay_types::{CriteriaLibrary, CriterionInput, GateWizardInput, GatesSpec, SpecPreconditions};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

// ── Sub-step enums ────────────────────────────────────────────────────────────

/// Which field within the criteria loop is currently active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CriteriaSubStep {
    Name,
    Description,
    Cmd,
    AddAnother,
}

/// Which field within the preconditions loop is currently active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreconditionSubStep {
    Ask,
    Requires,
    Commands,
}

// ── GateWizardState ───────────────────────────────────────────────────────────

/// Multi-step form state for the gate wizard.
#[allow(dead_code)] // criteria_edit_idx used in Plan 02 integration
pub struct GateWizardState {
    /// Current wizard step (0–6).
    pub(crate) step: usize,
    /// Per-step text input buffers (steps 0 and 1 use fields[0] and fields[1]).
    pub(crate) fields: Vec<Vec<String>>,
    /// Cursor position in active text input.
    pub(crate) cursor: usize,
    /// Gate slugs loaded at wizard open time (for the extends single-select, step 2).
    pub(crate) available_gates: Vec<String>,
    /// Ratatui list state for the extends step.
    pub(crate) extends_list_state: ListState,
    /// Criteria libraries loaded at wizard open time (for includes multi-select, step 3).
    pub(crate) available_libs: Vec<CriteriaLibrary>,
    /// Ratatui list state for the includes step.
    pub(crate) includes_list_state: ListState,
    /// Indices into `available_libs` that are toggled on.
    pub(crate) selected_includes: HashSet<usize>,
    /// Collected criteria so far.
    pub(crate) criteria: Vec<CriterionInput>,
    /// Active sub-step within the criteria loop.
    pub(crate) criteria_sub_step: CriteriaSubStep,
    /// Scratch fields for in-progress criterion: [name, description, cmd].
    pub(crate) criteria_scratch: [String; 3],
    /// Index into existing criteria being walked in edit mode.
    pub(crate) criteria_edit_idx: usize,
    /// Whether the user said "yes" to the preconditions prompt.
    pub(crate) preconditions_active: bool,
    /// Collected requires slugs.
    pub(crate) precondition_requires: Vec<String>,
    /// Collected command strings.
    pub(crate) precondition_commands: Vec<String>,
    /// Active sub-step within the preconditions section.
    pub(crate) precondition_sub_step: PreconditionSubStep,
    /// Scratch buffer for precondition input loop.
    pub(crate) precondition_scratch: String,
    /// None = create mode, Some(slug) = edit mode.
    pub(crate) edit_slug: Option<String>,
    /// Inline error display.
    pub error: Option<String>,
    /// Transient message when a list step is auto-skipped.
    pub(crate) auto_skip_msg: Option<String>,
}

impl GateWizardState {
    /// Create a fresh wizard at step 0 with empty field buffers.
    pub fn new(available_gates: Vec<String>, available_libs: Vec<CriteriaLibrary>) -> Self {
        let mut extends_list_state = ListState::default();
        extends_list_state.select(Some(0));

        let mut includes_list_state = ListState::default();
        if !available_libs.is_empty() {
            includes_list_state.select(Some(0));
        }

        GateWizardState {
            step: 0,
            fields: vec![vec![String::new()], vec![String::new()]],
            cursor: 0,
            available_gates,
            extends_list_state,
            available_libs,
            includes_list_state,
            selected_includes: HashSet::new(),
            criteria: Vec::new(),
            criteria_sub_step: CriteriaSubStep::Name,
            criteria_scratch: [String::new(), String::new(), String::new()],
            criteria_edit_idx: 0,
            preconditions_active: false,
            precondition_requires: Vec::new(),
            precondition_commands: Vec::new(),
            precondition_sub_step: PreconditionSubStep::Ask,
            precondition_scratch: String::new(),
            edit_slug: None,
            error: None,
            auto_skip_msg: None,
        }
    }

    /// Create a wizard pre-filled from an existing `GatesSpec` (edit mode).
    pub fn from_existing(
        gates: &GatesSpec,
        slug: String,
        available_gates: Vec<String>,
        available_libs: Vec<CriteriaLibrary>,
    ) -> Self {
        let mut state = Self::new(available_gates.clone(), available_libs.clone());

        // Pre-fill name (step 0) and description (step 1).
        state.fields[0] = vec![slug.clone()];
        state.cursor = slug.len();
        let desc = if gates.description.is_empty() {
            String::new()
        } else {
            gates.description.clone()
        };
        state.fields[1] = vec![desc];

        // Pre-select extends: index 0 = "(none)", index N+1 = available_gates[N].
        let extends_idx = match &gates.extends {
            None => 0,
            Some(ext) => available_gates
                .iter()
                .position(|g| g == ext)
                .map(|i| i + 1)
                .unwrap_or(0),
        };
        state.extends_list_state.select(Some(extends_idx));

        // Pre-select includes.
        for inc in &gates.include {
            if let Some(idx) = available_libs.iter().position(|l| &l.name == inc) {
                state.selected_includes.insert(idx);
            }
        }

        // Pre-fill criteria (convert GateCriterion → CriterionInput).
        state.criteria = gates
            .criteria
            .iter()
            .map(|c| CriterionInput {
                name: c.name.clone(),
                description: c.description.clone(),
                cmd: c.cmd.clone(),
            })
            .collect();

        // Pre-fill preconditions if any.
        if let Some(ref preconds) = gates.preconditions {
            state.preconditions_active = true;
            state.precondition_requires = preconds.requires.clone();
            state.precondition_commands = preconds.commands.clone();
        }

        state.edit_slug = Some(slug);
        state
    }
}

// ── GateWizardAction ──────────────────────────────────────────────────────────

/// Result produced by [`handle_gate_wizard_event`] after processing one key press.
#[derive(Debug)]
pub enum GateWizardAction {
    /// The user pressed a key that advanced or edited the form; re-render.
    Continue,
    /// The user completed all steps. The inner value holds the structured inputs
    /// ready to pass to `assay_core::wizard::apply_gate_wizard`.
    Submit(GateWizardInput),
    /// The user pressed Esc; all progress is discarded.
    Cancel,
}

// ── assemble_gate_input ───────────────────────────────────────────────────────

/// Assemble a [`GateWizardInput`] from the completed wizard state.
pub(crate) fn assemble_gate_input(state: &GateWizardState) -> GateWizardInput {
    let slug = state.fields[0]
        .last()
        .cloned()
        .unwrap_or_default()
        .trim()
        .to_string();

    let desc_str = state.fields[1].last().cloned().unwrap_or_default();
    let description = if desc_str.trim().is_empty() {
        None
    } else {
        Some(desc_str.trim().to_string())
    };

    // extends: index 0 = None, index N = available_gates[N-1].
    let extends = state.extends_list_state.selected().and_then(|i| {
        if i == 0 {
            None
        } else {
            state.available_gates.get(i - 1).cloned()
        }
    });

    let include: Vec<String> = state
        .selected_includes
        .iter()
        .filter_map(|&i| state.available_libs.get(i).map(|l| l.name.clone()))
        .collect();

    let preconditions = if state.preconditions_active
        && (!state.precondition_requires.is_empty() || !state.precondition_commands.is_empty())
    {
        Some(SpecPreconditions {
            requires: state.precondition_requires.clone(),
            commands: state.precondition_commands.clone(),
        })
    } else {
        None
    };

    GateWizardInput {
        slug,
        description,
        extends,
        include,
        criteria: state.criteria.clone(),
        preconditions,
        overwrite: state.edit_slug.is_some(),
    }
}

// ── Event handler ─────────────────────────────────────────────────────────────

/// Process a single key press and mutate `state` accordingly.
///
/// Returns [`GateWizardAction::Continue`] for most edits, [`GateWizardAction::Submit`]
/// when the user confirms the final step, and [`GateWizardAction::Cancel`] on Esc.
pub fn handle_gate_wizard_event(state: &mut GateWizardState, key: KeyEvent) -> GateWizardAction {
    // Esc at any step → cancel.
    if key.code == KeyCode::Esc {
        return GateWizardAction::Cancel;
    }

    // If auto_skip_msg is showing, any keypress clears it and advances.
    if state.auto_skip_msg.is_some() {
        state.auto_skip_msg = None;
        advance_step(state);
        return GateWizardAction::Continue;
    }

    match state.step {
        0 => handle_text_step(state, key, true),
        1 => handle_text_step(state, key, false),
        2 => handle_extends_step(state, key),
        3 => handle_includes_step(state, key),
        4 => handle_criteria_step(state, key),
        5 => handle_preconditions_step(state, key),
        6 => handle_confirm_step(state, key),
        _ => GateWizardAction::Continue,
    }
}

/// Handle a plain text-input step (steps 0 and 1).
/// `required` — if true, blank Enter shows an error instead of advancing.
fn handle_text_step(
    state: &mut GateWizardState,
    key: KeyEvent,
    required: bool,
) -> GateWizardAction {
    match key.code {
        KeyCode::Char(c) => {
            state.fields[state.step]
                .last_mut()
                .expect("fields[step] must have at least one element")
                .push(c);
            state.cursor += 1;
            state.error = None;
        }
        KeyCode::Backspace => {
            let line = state.fields[state.step]
                .last_mut()
                .expect("fields[step] must have at least one element");
            if !line.is_empty() {
                line.pop();
                if state.cursor > 0 {
                    state.cursor -= 1;
                }
            }
        }
        KeyCode::Enter => {
            let value = state.fields[state.step].last().cloned().unwrap_or_default();
            let trimmed = value.trim().to_string();
            if required && trimmed.is_empty() {
                state.error = Some("Gate name is required".to_string());
            } else {
                state.error = None;
                // Store trimmed value back.
                *state.fields[state.step]
                    .last_mut()
                    .expect("fields[step] must have at least one element") = trimmed;
                advance_step(state);
            }
        }
        _ => {}
    }
    GateWizardAction::Continue
}

/// Handle the extends step (step 2): single-select list.
fn handle_extends_step(state: &mut GateWizardState, key: KeyEvent) -> GateWizardAction {
    // +1 because index 0 is the "(none)" sentinel.
    let list_len = state.available_gates.len() + 1;

    match key.code {
        KeyCode::Up => {
            let sel = state.extends_list_state.selected().unwrap_or(0);
            state.extends_list_state.select(Some(sel.saturating_sub(1)));
        }
        KeyCode::Down => {
            let sel = state.extends_list_state.selected().unwrap_or(0);
            if sel + 1 < list_len {
                state.extends_list_state.select(Some(sel + 1));
            }
        }
        KeyCode::Enter => {
            advance_step(state);
        }
        _ => {}
    }
    GateWizardAction::Continue
}

/// Handle the includes step (step 3): multi-select list.
fn handle_includes_step(state: &mut GateWizardState, key: KeyEvent) -> GateWizardAction {
    let lib_count = state.available_libs.len();

    match key.code {
        KeyCode::Up => {
            let sel = state.includes_list_state.selected().unwrap_or(0);
            state
                .includes_list_state
                .select(Some(sel.saturating_sub(1)));
        }
        KeyCode::Down => {
            let sel = state.includes_list_state.selected().unwrap_or(0);
            if sel + 1 < lib_count {
                state.includes_list_state.select(Some(sel + 1));
            }
        }
        KeyCode::Char(' ') => {
            if let Some(sel) = state.includes_list_state.selected() {
                if state.selected_includes.contains(&sel) {
                    state.selected_includes.remove(&sel);
                } else {
                    state.selected_includes.insert(sel);
                }
            }
        }
        KeyCode::Enter => {
            advance_step(state);
        }
        _ => {}
    }
    GateWizardAction::Continue
}

/// Handle the criteria loop (step 4).
fn handle_criteria_step(state: &mut GateWizardState, key: KeyEvent) -> GateWizardAction {
    match &state.criteria_sub_step.clone() {
        CriteriaSubStep::Name => handle_criteria_name(state, key),
        CriteriaSubStep::Description => handle_criteria_description(state, key),
        CriteriaSubStep::Cmd => handle_criteria_cmd(state, key),
        CriteriaSubStep::AddAnother => handle_criteria_add_another(state, key),
    }
}

fn handle_criteria_name(state: &mut GateWizardState, key: KeyEvent) -> GateWizardAction {
    match key.code {
        KeyCode::Char(c) => {
            state.criteria_scratch[0].push(c);
            state.error = None;
        }
        KeyCode::Backspace => {
            state.criteria_scratch[0].pop();
        }
        KeyCode::Enter => {
            let name = state.criteria_scratch[0].trim().to_string();
            if name.is_empty() {
                // Blank name on first entry (not in edit walk) means done.
                // Move to AddAnother to decide.
                state.criteria_sub_step = CriteriaSubStep::AddAnother;
            } else {
                state.criteria_scratch[0] = name;
                state.criteria_sub_step = CriteriaSubStep::Description;
            }
        }
        _ => {}
    }
    GateWizardAction::Continue
}

fn handle_criteria_description(state: &mut GateWizardState, key: KeyEvent) -> GateWizardAction {
    match key.code {
        KeyCode::Char(c) => {
            state.criteria_scratch[1].push(c);
        }
        KeyCode::Backspace => {
            state.criteria_scratch[1].pop();
        }
        KeyCode::Enter => {
            state.criteria_sub_step = CriteriaSubStep::Cmd;
        }
        _ => {}
    }
    GateWizardAction::Continue
}

fn handle_criteria_cmd(state: &mut GateWizardState, key: KeyEvent) -> GateWizardAction {
    match key.code {
        KeyCode::Char(c) => {
            state.criteria_scratch[2].push(c);
        }
        KeyCode::Backspace => {
            state.criteria_scratch[2].pop();
        }
        KeyCode::Enter => {
            // Commit the criterion.
            let cmd_str = state.criteria_scratch[2].trim().to_string();
            let criterion = CriterionInput {
                name: state.criteria_scratch[0].clone(),
                description: state.criteria_scratch[1].clone(),
                cmd: if cmd_str.is_empty() {
                    None
                } else {
                    Some(cmd_str)
                },
            };
            state.criteria.push(criterion);
            // Reset scratch.
            state.criteria_scratch = [String::new(), String::new(), String::new()];
            state.criteria_sub_step = CriteriaSubStep::AddAnother;
        }
        _ => {}
    }
    GateWizardAction::Continue
}

fn handle_criteria_add_another(state: &mut GateWizardState, key: KeyEvent) -> GateWizardAction {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            state.criteria_sub_step = CriteriaSubStep::Name;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter => {
            // Done with criteria — advance to preconditions.
            advance_step(state);
        }
        _ => {}
    }
    GateWizardAction::Continue
}

/// Handle the preconditions section (step 5).
fn handle_preconditions_step(state: &mut GateWizardState, key: KeyEvent) -> GateWizardAction {
    match &state.precondition_sub_step.clone() {
        PreconditionSubStep::Ask => handle_precondition_ask(state, key),
        PreconditionSubStep::Requires => handle_precondition_requires(state, key),
        PreconditionSubStep::Commands => handle_precondition_commands(state, key),
    }
}

fn handle_precondition_ask(state: &mut GateWizardState, key: KeyEvent) -> GateWizardAction {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            state.preconditions_active = true;
            state.precondition_sub_step = PreconditionSubStep::Requires;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Enter => {
            state.preconditions_active = false;
            advance_step(state);
        }
        _ => {}
    }
    GateWizardAction::Continue
}

fn handle_precondition_requires(state: &mut GateWizardState, key: KeyEvent) -> GateWizardAction {
    match key.code {
        KeyCode::Char(c) => {
            state.precondition_scratch.push(c);
        }
        KeyCode::Backspace => {
            state.precondition_scratch.pop();
        }
        KeyCode::Enter => {
            let val = state.precondition_scratch.trim().to_string();
            if val.is_empty() {
                // Done collecting requires — move to commands.
                state.precondition_scratch = String::new();
                state.precondition_sub_step = PreconditionSubStep::Commands;
            } else {
                state.precondition_requires.push(val);
                state.precondition_scratch = String::new();
            }
        }
        _ => {}
    }
    GateWizardAction::Continue
}

fn handle_precondition_commands(state: &mut GateWizardState, key: KeyEvent) -> GateWizardAction {
    match key.code {
        KeyCode::Char(c) => {
            state.precondition_scratch.push(c);
        }
        KeyCode::Backspace => {
            state.precondition_scratch.pop();
        }
        KeyCode::Enter => {
            let val = state.precondition_scratch.trim().to_string();
            if val.is_empty() {
                // Done — advance to confirm.
                state.precondition_scratch = String::new();
                advance_step(state);
            } else {
                state.precondition_commands.push(val);
                state.precondition_scratch = String::new();
            }
        }
        _ => {}
    }
    GateWizardAction::Continue
}

/// Handle the confirm step (step 6).
fn handle_confirm_step(state: &mut GateWizardState, key: KeyEvent) -> GateWizardAction {
    match key.code {
        KeyCode::Enter => GateWizardAction::Submit(assemble_gate_input(state)),
        _ => GateWizardAction::Continue,
    }
}

/// Advance `step` by one, handling auto-skip for empty lists at steps 2 and 3.
fn advance_step(state: &mut GateWizardState) {
    let next = state.step + 1;
    state.step = next;
    state.cursor = 0;
    state.error = None;

    // Check for auto-skip conditions.
    if next == 2 && state.available_gates.is_empty() {
        state.auto_skip_msg =
            Some("No gate specs available to extend. Step skipped. (press any key)".to_string());
    } else if next == 3 && state.available_libs.is_empty() {
        state.auto_skip_msg = Some(
            "No criteria libraries available to include. Step skipped. (press any key)".to_string(),
        );
    }
}

// ── Renderer ──────────────────────────────────────────────────────────────────

const KEY_HINT: &str = "Enter: advance · Esc: cancel";

/// Render the gate wizard form into `frame`.
///
/// Takes `&mut GateWizardState` because `render_stateful_widget` requires `&mut ListState`.
pub fn draw_gate_wizard(frame: &mut Frame, area: Rect, state: &mut GateWizardState) {
    let [header_area, main_area, hint_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
    ])
    .areas(area);

    // ── Header ────────────────────────────────────────────────────────────────
    let title = if let Some(ref slug) = state.edit_slug.clone() {
        format!(" Edit Gate: {slug} ")
    } else {
        " Gate Wizard ".to_string()
    };
    let step_display = format!("Step {}/7", state.step + 1);
    let prompt = step_prompt(state);

    let header_text = Text::from(vec![
        Line::from(step_display).style(Style::default().dim()),
        Line::from(prompt).bold(),
    ]);
    let header =
        Paragraph::new(header_text).block(Block::default().title(title).borders(Borders::ALL));
    frame.render_widget(header, header_area);

    // ── Auto-skip message ──────────────────────────────────────────────────────
    if let Some(ref msg) = state.auto_skip_msg.clone() {
        let skip_text = Text::from(vec![Line::from(Span::styled(
            msg.as_str(),
            Style::default().dim(),
        ))]);
        let skip = Paragraph::new(skip_text).block(Block::default().borders(Borders::ALL));
        frame.render_widget(skip, main_area);

        let hint = Paragraph::new(Text::from(Line::from(Span::styled(
            KEY_HINT,
            Style::default().dim(),
        ))))
        .block(Block::default().borders(Borders::ALL));
        frame.render_widget(hint, hint_area);
        return;
    }

    // ── Main content ──────────────────────────────────────────────────────────
    match state.step {
        0 | 1 => {
            let current = state.fields[state.step].last().cloned().unwrap_or_default();
            let input_text = Text::from(Line::from(format!("{current}_")));
            let input = Paragraph::new(input_text).block(Block::default().borders(Borders::ALL));
            frame.render_widget(input, main_area);
        }

        2 => {
            // Extends single-select.
            let mut items: Vec<ListItem> = vec![ListItem::new("(none)")];
            for gate in &state.available_gates {
                items.push(ListItem::new(gate.as_str()));
            }
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL))
                .highlight_style(Style::default().bold().reversed());
            frame.render_stateful_widget(list, main_area, &mut state.extends_list_state);
        }

        3 => {
            // Includes multi-select.
            let selected = state.selected_includes.clone();
            let items: Vec<ListItem> = state
                .available_libs
                .iter()
                .enumerate()
                .map(|(i, lib)| {
                    let prefix = if selected.contains(&i) {
                        "[x] "
                    } else {
                        "[ ] "
                    };
                    ListItem::new(format!("{prefix}{}", lib.name))
                })
                .collect();
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL))
                .highlight_style(Style::default().bold().reversed());
            frame.render_stateful_widget(list, main_area, &mut state.includes_list_state);
        }

        4 => {
            // Criteria loop.
            let mut lines: Vec<Line> = Vec::new();

            // Show already-committed criteria.
            for c in &state.criteria {
                let cmd_part = c
                    .cmd
                    .as_deref()
                    .map(|s| format!(" [{s}]"))
                    .unwrap_or_default();
                lines.push(Line::from(format!("  • {}{cmd_part}", c.name)));
            }

            // Show active sub-step prompt.
            let active_val = match &state.criteria_sub_step {
                CriteriaSubStep::Name => {
                    lines.push(Line::from(""));
                    Line::from(format!("Name: {}_", state.criteria_scratch[0]))
                }
                CriteriaSubStep::Description => {
                    Line::from(format!("Description: {}_", state.criteria_scratch[1]))
                }
                CriteriaSubStep::Cmd => Line::from(format!(
                    "Command (Enter to skip): {}_",
                    state.criteria_scratch[2]
                )),
                CriteriaSubStep::AddAnother => Line::from("Add another criterion? (y/N)"),
            };
            lines.push(active_val);

            let input =
                Paragraph::new(Text::from(lines)).block(Block::default().borders(Borders::ALL));
            frame.render_widget(input, main_area);
        }

        5 => {
            // Preconditions.
            let content_line = match &state.precondition_sub_step {
                PreconditionSubStep::Ask => Line::from("Add preconditions? (y/N)"),
                PreconditionSubStep::Requires => {
                    let mut lines: Vec<Line> = state
                        .precondition_requires
                        .iter()
                        .map(|r| Line::from(format!("  requires: {r}")))
                        .collect();
                    lines.push(Line::from(format!(
                        "Require spec slug (blank to finish): {}_",
                        state.precondition_scratch
                    )));
                    let input = Paragraph::new(Text::from(lines))
                        .block(Block::default().borders(Borders::ALL));
                    frame.render_widget(input, main_area);
                    render_hint_bar(frame, hint_area, &state.error);
                    return;
                }
                PreconditionSubStep::Commands => {
                    let mut lines: Vec<Line> = state
                        .precondition_commands
                        .iter()
                        .map(|c| Line::from(format!("  command: {c}")))
                        .collect();
                    lines.push(Line::from(format!(
                        "Shell command (blank to finish): {}_",
                        state.precondition_scratch
                    )));
                    let input = Paragraph::new(Text::from(lines))
                        .block(Block::default().borders(Borders::ALL));
                    frame.render_widget(input, main_area);
                    render_hint_bar(frame, hint_area, &state.error);
                    return;
                }
            };
            let input = Paragraph::new(Text::from(content_line))
                .block(Block::default().borders(Borders::ALL));
            frame.render_widget(input, main_area);
        }

        6 => {
            // Confirm — show summary.
            let slug = state.fields[0].last().cloned().unwrap_or_default();
            let desc = state.fields[1].last().cloned().unwrap_or_default();
            let desc_display = if desc.is_empty() {
                "(none)".to_string()
            } else {
                desc
            };

            let extends_display = state
                .extends_list_state
                .selected()
                .and_then(|i| {
                    if i == 0 {
                        None
                    } else {
                        state.available_gates.get(i - 1)
                    }
                })
                .map(|g| g.as_str())
                .unwrap_or("(none)");

            let includes_display: Vec<&str> = state
                .selected_includes
                .iter()
                .filter_map(|&i| state.available_libs.get(i).map(|l| l.name.as_str()))
                .collect();
            let includes_str = if includes_display.is_empty() {
                "(none)".to_string()
            } else {
                includes_display.join(", ")
            };

            let precond_summary = if state.preconditions_active {
                format!(
                    "{} requires, {} commands",
                    state.precondition_requires.len(),
                    state.precondition_commands.len()
                )
            } else {
                "(none)".to_string()
            };

            let lines = vec![
                Line::from(format!("  Name:           {slug}")),
                Line::from(format!("  Description:    {desc_display}")),
                Line::from(format!("  Extends:        {extends_display}")),
                Line::from(format!("  Includes:       {includes_str}")),
                Line::from(format!(
                    "  Criteria:       {} defined",
                    state.criteria.len()
                )),
                Line::from(format!("  Preconditions:  {precond_summary}")),
                Line::from(""),
                Line::from("Press Enter to create gate, Esc to cancel."),
            ];
            let summary = Paragraph::new(Text::from(lines))
                .block(Block::default().title(" Confirm ").borders(Borders::ALL));
            frame.render_widget(summary, main_area);
        }

        _ => {}
    }

    render_hint_bar(frame, hint_area, &state.error);
}

/// Render the hint / error bar at the bottom.
fn render_hint_bar(frame: &mut Frame, area: Rect, error: &Option<String>) {
    let hint_text = if let Some(err) = error {
        Text::from(Line::from(Span::styled(
            err.as_str(),
            Style::default().fg(Color::Red),
        )))
    } else {
        Text::from(Line::from(Span::styled(KEY_HINT, Style::default().dim())))
    };
    let hint = Paragraph::new(hint_text).block(Block::default().borders(Borders::ALL));
    frame.render_widget(hint, area);
}

/// Return the prompt label string for the given step.
fn step_prompt(state: &GateWizardState) -> String {
    match state.step {
        0 => "Gate name (slug):".to_string(),
        1 => "Description (optional):".to_string(),
        2 => "Extends: select parent gate (Up/Down/Enter):".to_string(),
        3 => "Includes: toggle libraries (Space/Up/Down/Enter):".to_string(),
        4 => match &state.criteria_sub_step {
            CriteriaSubStep::Name => "Criteria — criterion name (blank to finish):".to_string(),
            CriteriaSubStep::Description => "Criteria — description:".to_string(),
            CriteriaSubStep::Cmd => "Criteria — shell command (optional):".to_string(),
            CriteriaSubStep::AddAnother => "Criteria — add another? (y/N):".to_string(),
        },
        5 => match &state.precondition_sub_step {
            PreconditionSubStep::Ask => "Preconditions — add any? (y/N):".to_string(),
            PreconditionSubStep::Requires => {
                "Preconditions — required gate slug (blank to finish):".to_string()
            }
            PreconditionSubStep::Commands => {
                "Preconditions — shell command (blank to finish):".to_string()
            }
        },
        6 => "Review and confirm:".to_string(),
        _ => String::new(),
    }
}
