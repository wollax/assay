//! In-TUI authoring wizard: state machine, event handler, and renderer.
//!
//! This module provides the multi-step form that collects milestone name,
//! description, chunk count, per-chunk names, and per-chunk criteria.
//! On completion it returns `WizardAction::Submit(WizardInputs)` so the
//! caller can invoke `assay_core::wizard::create_from_inputs`.

use assay_core::wizard::{CriterionInput, WizardChunkInput, WizardInputs, slugify};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Paragraph};

// ── Public types ──────────────────────────────────────────────────────────────

/// Multi-step form state for the authoring wizard.
///
/// `fields` holds one `Vec<String>` per step; multi-line steps (criteria) accumulate
/// entries in the inner `Vec`. `cursor` is the current insertion position within the
/// active single-line buffer. `error` carries the last `create_from_inputs` failure
/// message so `draw_wizard` can render it inline without panicking.
pub struct WizardState {
    /// Current step index (0 = milestone name, 1 = description, 2 = chunk count,
    /// 3.. = alternating chunk-name / chunk-criteria steps).
    pub step: usize,
    /// Per-step field buffers. Each inner `Vec<String>` holds the line(s) entered so far.
    ///
    /// Invariant: `fields.len() == step + 1` and `fields[step].len() >= 1` at all times.
    /// Only `handle_wizard_event` may mutate these to preserve the invariant.
    pub(crate) fields: Vec<Vec<String>>,
    /// Insertion-point offset within the active input line.
    /// Maintained for future mid-line editing support; currently always equals
    /// `current_line(state).len()` because only append/remove-from-end is supported.
    pub(crate) cursor: usize,
    /// Number of chunks the user entered in the chunk-count step.
    pub(crate) chunk_count: usize,
    /// Inline error message set by field validation or a failed `create_from_inputs` call.
    pub error: Option<String>,
    /// When `true`, the current input line in a criteria step is collecting a command
    /// rather than a criterion name. This flag enables the name→cmd sub-step alternation
    /// without changing the step index arithmetic.
    pub criteria_awaiting_cmd: bool,
}

impl WizardState {
    /// Create a fresh wizard at step 0 with empty field buffers.
    pub fn new() -> Self {
        WizardState {
            step: 0,
            fields: vec![vec![String::new()]],
            cursor: 0,
            chunk_count: 0,
            error: None,
            criteria_awaiting_cmd: false,
        }
    }
}

impl Default for WizardState {
    fn default() -> Self {
        Self::new()
    }
}

/// The result produced by [`handle_wizard_event`] after processing one key press.
#[derive(Debug)]
pub enum WizardAction {
    /// The user pressed a key that advanced or edited the form; re-render.
    Continue,
    /// The user completed all steps; the inner value holds the structured inputs
    /// ready to pass to `assay_core::wizard::create_from_inputs`.
    Submit(WizardInputs),
    /// The user pressed Esc; all progress is discarded and the caller should
    /// return to the dashboard screen.
    Cancel,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Return the active input buffer (the last line of the current step's field vec).
///
/// Panics if the `fields[step].len() >= 1` invariant is violated — consistent
/// with the `expect` calls in the Backspace handler that enforce the same invariant.
fn current_line(state: &WizardState) -> &str {
    state.fields[state.step]
        .last()
        .expect("fields[step] must contain at least one element — invariant violated")
        .as_str()
}

/// Assemble a [`WizardInputs`] from the completed wizard state.
///
/// Called when the user presses blank Enter on the last chunk's criteria step.
/// Strips the trailing empty string that the blank-Enter transition leaves behind.
fn assemble_inputs(state: &WizardState) -> WizardInputs {
    let name = state.fields[0].last().cloned().unwrap_or_default();
    let slug = slugify(&name);

    let description_str = state.fields[1].last().cloned().unwrap_or_default();
    let description = if description_str.is_empty() {
        None
    } else {
        Some(description_str)
    };

    let mut chunks = Vec::with_capacity(state.chunk_count);
    for i in 0..state.chunk_count {
        let name_step = 3 + 2 * i;
        let criteria_step = 3 + 2 * i + 1;

        let chunk_name = state.fields[name_step].last().cloned().unwrap_or_default();
        let chunk_slug = slugify(&chunk_name);

        // Fields in the criteria step alternate: name, cmd, name, cmd, ..., trailing-empty.
        // Iterate in pairs to build CriterionInput values.
        let raw = &state.fields[criteria_step];
        let mut criteria = Vec::new();
        let mut j = 0;
        while j + 1 < raw.len() {
            let crit_name = &raw[j];
            let crit_cmd = &raw[j + 1];
            if !crit_name.is_empty() {
                criteria.push(CriterionInput {
                    name: crit_name.clone(),
                    description: String::new(),
                    cmd: if crit_cmd.is_empty() {
                        None
                    } else {
                        Some(crit_cmd.clone())
                    },
                });
            }
            j += 2;
        }

        chunks.push(WizardChunkInput {
            slug: chunk_slug,
            name: chunk_name,
            criteria,
        });
    }

    WizardInputs {
        slug,
        name,
        description,
        chunks,
    }
}

// ── Event handler ─────────────────────────────────────────────────────────────

/// Process a single key press and mutate `state` accordingly.
///
/// Returns [`WizardAction::Continue`] for most edits, [`WizardAction::Submit`]
/// when the final blank-Enter terminates the last chunk's criteria, and
/// [`WizardAction::Cancel`] when the user presses Esc.
pub fn handle_wizard_event(state: &mut WizardState, event: KeyEvent) -> WizardAction {
    match event.code {
        // ── Esc: cancel ───────────────────────────────────────────────────────
        KeyCode::Esc => WizardAction::Cancel,

        // ── Char: append to current buffer ────────────────────────────────────
        KeyCode::Char(c) if state.step == 2 => {
            // Chunk count step: only accept digits 1–7; replace (not append).
            if ('1'..='7').contains(&c) {
                state.fields[2] = vec![c.to_string()];
                state.cursor = 1;
                state.error = None;
            }
            WizardAction::Continue
        }

        KeyCode::Char(c) => {
            state.fields[state.step]
                .last_mut()
                .expect("fields[step] must always have at least one element")
                .push(c);
            state.cursor += 1;
            state.error = None;
            WizardAction::Continue
        }

        // ── Backspace ─────────────────────────────────────────────────────────
        KeyCode::Backspace => {
            let line_empty = current_line(state).is_empty();

            if line_empty && state.step > 0 {
                // Go back to previous step.
                state.step -= 1;
                state.fields.pop();
                state.cursor = state.fields[state.step]
                    .last()
                    .map(|s| s.len())
                    .unwrap_or(0);
            } else if !line_empty {
                state.fields[state.step]
                    .last_mut()
                    .expect("fields[step] is non-empty")
                    .pop();
                if state.cursor > 0 {
                    state.cursor -= 1;
                }
            }
            // step == 0 and empty: do nothing.
            WizardAction::Continue
        }

        // ── Enter: advance / submit ───────────────────────────────────────────
        KeyCode::Enter => handle_enter(state),

        // ── All other keys: ignore ────────────────────────────────────────────
        _ => WizardAction::Continue,
    }
}

/// Handle Enter key logic, factored out for clarity.
fn handle_enter(state: &mut WizardState) -> WizardAction {
    let step = state.step;

    match step {
        // Step 0: milestone name (required).
        0 => {
            if current_line(state).is_empty() {
                state.error = Some("Milestone name is required".to_string());
            } else if slugify(current_line(state)).is_empty() {
                state.error = Some("Name must contain at least one letter or digit".to_string());
            } else {
                state.step = 1;
                state.fields.push(vec![String::new()]);
                state.cursor = 0;
                state.error = None;
            }
            WizardAction::Continue
        }

        // Step 1: description (optional — blank OK).
        1 => {
            state.step = 2;
            state.fields.push(vec![String::new()]);
            state.cursor = 0;
            state.error = None;
            WizardAction::Continue
        }

        // Step 2: chunk count (1–7).
        2 => {
            let line = current_line(state).to_string();
            match line.parse::<usize>() {
                Ok(n) if (1..=7).contains(&n) => {
                    state.chunk_count = n;
                    state.step = 3;
                    state.fields.push(vec![String::new()]);
                    state.cursor = 0;
                    state.error = None;
                }
                _ => {
                    state.error = Some("Enter a number from 1 to 7".to_string());
                }
            }
            WizardAction::Continue
        }

        // Steps 3+: alternating chunk-name / chunk-criteria.
        step => {
            let offset = step - 3;
            let i = offset / 2;
            let is_criteria = offset % 2 == 1;

            if !is_criteria {
                // Chunk name step.
                if current_line(state).is_empty() {
                    state.error = Some("Chunk name is required".to_string());
                    WizardAction::Continue
                } else if slugify(current_line(state)).is_empty() {
                    state.error =
                        Some("Chunk name must contain at least one letter or digit".to_string());
                    WizardAction::Continue
                } else {
                    state.step += 1;
                    state.fields.push(vec![String::new()]);
                    state.cursor = 0;
                    state.error = None;
                    WizardAction::Continue
                }
            } else if state.criteria_awaiting_cmd {
                // Cmd sub-step: store the cmd value, return to name phase.
                // Current line is the cmd (may be empty → None later).
                // Push a new empty string for the next criterion name.
                state.criteria_awaiting_cmd = false;
                state.fields[step].push(String::new());
                state.cursor = 0;
                WizardAction::Continue
            } else {
                // Criteria name sub-step.
                if !current_line(state).is_empty() {
                    // Non-empty Enter: criterion name entered.
                    // Switch to cmd sub-step: push a new empty string for cmd input.
                    state.criteria_awaiting_cmd = true;
                    state.fields[step].push(String::new());
                    state.cursor = 0;
                    WizardAction::Continue
                } else {
                    // Blank Enter: done with criteria for this chunk.
                    if i < state.chunk_count - 1 {
                        // More chunks to go: advance to next chunk's name step.
                        state.step += 1;
                        state.fields.push(vec![String::new()]);
                        state.cursor = 0;
                        state.error = None;
                        WizardAction::Continue
                    } else {
                        // Last chunk: assemble and submit.
                        let inputs = assemble_inputs(state);
                        WizardAction::Submit(inputs)
                    }
                }
            }
        }
    }
}

// ── Renderer ──────────────────────────────────────────────────────────────────

const KEY_HINT: &str = "Enter to confirm · Backspace to go back · Esc to cancel";

/// Render the wizard form into `frame`.
///
/// Shows the current step prompt, active field buffer, a dim slug-preview hint
/// below name fields, and any inline error from the last failed submission.
pub fn draw_wizard(frame: &mut Frame, area: Rect, state: &WizardState) {
    // Vertical split: header (3), main input (fill), hint/error (3).
    let [header_area, input_area, hint_area] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Fill(1),
        Constraint::Length(3),
    ])
    .areas(area);

    // ── Total steps ──────────────────────────────────────────────────────────
    // Use 7 as the placeholder total when chunk_count is 0 (user hasn't chosen yet).
    // 7 = 3 base steps (name, description, chunk count) + 2 × 2 (two chunks, name + criteria
    // each) — a representative estimate for the most common case until the user picks a count.
    let total_steps = if state.chunk_count > 0 {
        3 + 2 * state.chunk_count
    } else {
        7
    };

    // ── Prompt label for current step ────────────────────────────────────────
    let prompt = step_prompt(state.step, state.chunk_count, state.criteria_awaiting_cmd);

    // ── Header: "New Milestone" title + step counter + prompt ────────────────
    let header_text = Text::from(vec![
        Line::from(format!("Step {} of {}", state.step + 1, total_steps))
            .style(Style::default().dim()),
        Line::from(prompt).bold(),
    ]);
    let header = Paragraph::new(header_text).block(
        Block::default()
            .title(" New Milestone ")
            .borders(Borders::ALL),
    );
    frame.render_widget(header, header_area);

    // ── Main input area ───────────────────────────────────────────────────────
    let is_criteria_step = state.step >= 3 && (state.step - 3) % 2 == 1;

    let input_text = if is_criteria_step {
        // Show committed criteria lines above the active line.
        let fields = &state.fields[state.step];
        let committed: Vec<Line> = fields[..fields.len() - 1]
            .iter()
            .map(|s| Line::from(s.as_str()))
            .collect();
        let active_line = Line::from(format!("{}_", current_line(state)));
        let mut lines = committed;
        lines.push(active_line);
        Text::from(lines)
    } else {
        Text::from(Line::from(format!("{}_", current_line(state))))
    };

    let input = Paragraph::new(input_text).block(Block::default().borders(Borders::ALL));
    frame.render_widget(input, input_area);

    // ── Hint / error area ─────────────────────────────────────────────────────
    let hint_text = if let Some(ref err) = state.error {
        Text::from(Line::from(Span::styled(
            err.as_str(),
            Style::default().fg(Color::Red),
        )))
    } else {
        // Slug preview for name steps; keyboard hint for others.
        let is_name_step =
            state.step == 0 || (state.step >= 3 && (state.step - 3).is_multiple_of(2));

        if is_name_step {
            let name = current_line(state);
            let preview = if name.is_empty() {
                "→ slug: (type a name)".to_string()
            } else {
                let slug = slugify(name);
                if slug.is_empty() {
                    "→ slug: (name must contain at least one letter or digit)".to_string()
                } else {
                    format!("→ slug: {}", slug)
                }
            };
            let mut lines = vec![Line::from(Span::styled(preview, Style::default().dim()))];
            lines.push(Line::from(Span::styled(KEY_HINT, Style::default().dim())));
            Text::from(lines)
        } else {
            Text::from(Line::from(Span::styled(KEY_HINT, Style::default().dim())))
        }
    };

    let hint = Paragraph::new(hint_text).block(Block::default().borders(Borders::ALL));
    frame.render_widget(hint, hint_area);
}

/// Return the prompt label string for the given step.
fn step_prompt(step: usize, _chunk_count: usize, awaiting_cmd: bool) -> String {
    match step {
        0 => "Milestone name:".to_string(),
        1 => "Description (optional):".to_string(),
        2 => "Number of chunks (1–7):".to_string(),
        _ => {
            let offset = step - 3;
            let i = offset / 2;
            let is_criteria = offset % 2 == 1;
            if is_criteria {
                if awaiting_cmd {
                    "Command (Enter to skip):".to_string()
                } else {
                    format!("Chunk {} criteria (blank line when done):", i + 1)
                }
            } else {
                format!("Chunk {} name:", i + 1)
            }
        }
    }
}
