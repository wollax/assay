//! Slash command overlay: parsing, dispatch, state, event handling, and rendering.
//!
//! Entry points: [`parse_slash_cmd`], [`execute_slash_cmd`], [`handle_slash_event`],
//! [`draw_slash_overlay`]. All functions are free functions with no trait objects (D001).
//! Command dispatch is synchronous and in-process (D111).

use std::path::Path;

use crossterm::event::KeyEvent;
use ratatui::layout::Rect;

// ── SlashCmd ──────────────────────────────────────────────────────────────────

/// Known slash commands the user can invoke from the overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlashCmd {
    GateCheck,
    GateWizard,
    GateEdit(String),
    Status,
    NextChunk,
    SpecShow,
    PrCreate,
}

/// Lookup table used for parsing and tab completion.
///
/// Note: `GateEdit(String)` carries data and cannot appear in this table.
/// It is handled by the parameterized-command path in [`parse_slash_cmd`].
pub const COMMANDS: &[(&str, SlashCmd)] = &[
    ("gate-check", SlashCmd::GateCheck),
    ("gate-wizard", SlashCmd::GateWizard),
    ("next-chunk", SlashCmd::NextChunk),
    ("pr-create", SlashCmd::PrCreate),
    ("spec-show", SlashCmd::SpecShow),
    ("status", SlashCmd::Status),
];

// ── SlashState ────────────────────────────────────────────────────────────────

/// Mutable state for the slash command overlay.
#[derive(Debug, Default, Clone)]
pub struct SlashState {
    /// Current text in the input buffer (without leading `/`).
    pub input: String,
    /// Tab-completion suggestion (full command name with `/` prefix).
    pub suggestion: Option<String>,
    /// Result string from the last executed command.
    pub result: Option<String>,
    /// Error string from the last executed command.
    pub error: Option<String>,
}

// ── SlashAction ───────────────────────────────────────────────────────────────

/// Actions returned by the event handler to drive the outer app state machine.
#[derive(Debug, PartialEq, Eq)]
pub enum SlashAction {
    /// Keep the overlay open, no side-effects.
    Continue,
    /// Close the overlay without executing.
    Close,
    /// Execute the given command.
    Execute(SlashCmd),
}

// ── parse_slash_cmd ───────────────────────────────────────────────────────────

/// Parse a user-entered string into a `SlashCmd`.
///
/// Strips a leading `/` if present, trims whitespace, and matches
/// case-insensitively against known command names.
///
/// Parameterized commands (e.g. `gate-edit <slug>`) are handled before the
/// COMMANDS table lookup because they carry data in a tuple variant.
pub fn parse_slash_cmd(input: &str) -> Option<SlashCmd> {
    let trimmed = input
        .trim()
        .strip_prefix('/')
        .unwrap_or(input.trim())
        .trim();

    // Parameterized commands first (cannot use COMMANDS table due to carried data).
    if let Some(rest) = trimmed.strip_prefix("gate-edit") {
        let arg = rest.trim().to_string();
        return Some(SlashCmd::GateEdit(arg));
    }

    // Exact-match lookup (case-insensitive for the COMMANDS table).
    let lower = trimmed.to_lowercase();
    COMMANDS
        .iter()
        .find(|(name, _)| *name == lower)
        .map(|(_, cmd)| cmd.clone())
}

// ── tab_complete ──────────────────────────────────────────────────────────────

/// Prefix-match against known commands and return a suggestion.
///
/// The `input` should be the raw text *without* the leading `/`.
/// Returns the full command name with `/` prefix if a unique or first
/// alphabetical match is found; `None` if no commands match.
///
/// Parameterized commands not in COMMANDS (e.g. `gate-edit`) are checked first
/// so that `/gate-e` completes to `/gate-edit ` (trailing space for argument).
pub fn tab_complete(input: &str) -> Option<String> {
    let lower = input.trim().to_lowercase();
    if lower.is_empty() {
        return None;
    }

    // COMMANDS table match takes priority (alphabetical, exact-match table commands).
    let matches: Vec<&str> = COMMANDS
        .iter()
        .filter(|(name, _)| name.starts_with(&lower))
        .map(|(name, _)| *name)
        .collect();
    if let Some(name) = matches.first() {
        return Some(format!("/{name}"));
    }

    // Parameterized command prefixes (not in COMMANDS table) — checked after COMMANDS
    // so they only complete when no table command matches the prefix.
    if "gate-edit".starts_with(&lower) {
        return Some("/gate-edit ".to_string()); // trailing space ready for argument
    }

    None
}

// ── execute_slash_cmd ─────────────────────────────────────────────────────────

/// Dispatch a parsed command against assay-core functions.
///
/// All calls are synchronous. Errors are caught and returned as human-readable
/// strings — this function never panics.
pub fn execute_slash_cmd(cmd: SlashCmd, project_root: &Path) -> String {
    let assay_dir = project_root.join(".assay");
    let specs_dir = assay_dir.join("specs");

    match cmd {
        SlashCmd::GateWizard => "Opening gate wizard...".to_string(),

        SlashCmd::GateEdit(ref slug) => {
            if slug.is_empty() {
                return "Usage: /gate-edit <slug>".to_string();
            }
            format!("Opening gate editor for '{slug}'...")
        }

        SlashCmd::Status => match assay_core::milestone::cycle_status(&assay_dir) {
            Ok(Some(cs)) => {
                let chunk_info = cs
                    .active_chunk_slug
                    .as_deref()
                    .unwrap_or("none (all complete)");
                format!(
                    "Milestone: {} ({})\nActive chunk: {}\nProgress: {}/{}",
                    cs.milestone_name,
                    cs.milestone_slug,
                    chunk_info,
                    cs.completed_count,
                    cs.total_count,
                )
            }
            Ok(None) => "No milestone currently in progress.".to_string(),
            Err(e) => format!("Error loading status: {e}"),
        },

        SlashCmd::NextChunk => {
            let status = match assay_core::milestone::cycle_status(&assay_dir) {
                Ok(Some(cs)) => cs,
                Ok(None) => return "No milestone currently in progress.".to_string(),
                Err(e) => return format!("Error loading status: {e}"),
            };
            let chunk_slug = match &status.active_chunk_slug {
                Some(slug) => slug.clone(),
                None => return "All chunks complete — no next chunk.".to_string(),
            };
            match assay_core::spec::load_spec_entry_with_diagnostics(&chunk_slug, &specs_dir) {
                Ok(entry) => {
                    let spec_name = match &entry {
                        assay_core::spec::SpecEntry::Directory { gates, .. } => gates.name.clone(),
                        assay_core::spec::SpecEntry::Legacy { slug, .. } => slug.clone(),
                    };
                    format!(
                        "Next chunk: {} (spec: {})\nMilestone: {} ({}/{})",
                        chunk_slug,
                        spec_name,
                        status.milestone_slug,
                        status.completed_count,
                        status.total_count,
                    )
                }
                Err(e) => format!("Next chunk: {chunk_slug}\nError loading spec: {e}"),
            }
        }

        SlashCmd::GateCheck => {
            let status = match assay_core::milestone::cycle_status(&assay_dir) {
                Ok(Some(cs)) => cs,
                Ok(None) => return "No milestone currently in progress.".to_string(),
                Err(e) => return format!("Error loading status: {e}"),
            };
            match assay_core::pr::pr_check_milestone_gates(
                &assay_dir,
                &specs_dir,
                project_root,
                &status.milestone_slug,
            ) {
                Ok(failures) if failures.is_empty() => {
                    "All gates pass for current milestone.".to_string()
                }
                Ok(failures) => {
                    let mut out = String::from("Gate failures:\n");
                    for f in &failures {
                        out.push_str(&format!(
                            "  {} — {} required criteria failed\n",
                            f.chunk_slug, f.required_failed
                        ));
                    }
                    out
                }
                Err(e) => format!("Error evaluating gates: {e}"),
            }
        }

        SlashCmd::SpecShow => {
            let status = match assay_core::milestone::cycle_status(&assay_dir) {
                Ok(Some(cs)) => cs,
                Ok(None) => return "No milestone currently in progress.".to_string(),
                Err(e) => return format!("Error loading status: {e}"),
            };
            let chunk_slug = match &status.active_chunk_slug {
                Some(slug) => slug.clone(),
                None => return "All chunks complete — no active spec to show.".to_string(),
            };
            match assay_core::spec::load_spec_entry_with_diagnostics(&chunk_slug, &specs_dir) {
                Ok(assay_core::spec::SpecEntry::Directory { gates, .. }) => {
                    let mut out = format!("Spec: {}\nCriteria:\n", gates.name);
                    for c in &gates.criteria {
                        out.push_str(&format!("  • {} — {}\n", c.name, c.description));
                    }
                    out
                }
                Ok(assay_core::spec::SpecEntry::Legacy { slug, .. }) => {
                    format!("Legacy spec: {slug} (criteria not available in directory format)")
                }
                Err(e) => format!("Error loading spec for '{chunk_slug}': {e}"),
            }
        }

        SlashCmd::PrCreate => {
            let status = match assay_core::milestone::cycle_status(&assay_dir) {
                Ok(Some(cs)) => cs,
                Ok(None) => return "No milestone currently in progress.".to_string(),
                Err(e) => return format!("Error loading status: {e}"),
            };
            match assay_core::pr::pr_check_milestone_gates(
                &assay_dir,
                &specs_dir,
                project_root,
                &status.milestone_slug,
            ) {
                Ok(failures) if failures.is_empty() => {
                    format!(
                        "Gates pass for '{}'. Run `assay pr create` to open the PR.",
                        status.milestone_slug
                    )
                }
                Ok(failures) => {
                    let slugs: Vec<&str> = failures.iter().map(|f| f.chunk_slug.as_str()).collect();
                    format!("Cannot create PR — gate failures in: {}", slugs.join(", "))
                }
                Err(e) => format!("Error checking gates for PR: {e}"),
            }
        }
    }
}

// ── handle_slash_event ────────────────────────────────────────────────────────

/// Handle a key event within the slash overlay.
///
/// Processes character input, Backspace, Tab completion, Enter dispatch, and
/// Esc to close. Returns a `SlashAction` that the caller (`App::handle_event`)
/// uses to drive state transitions.
pub fn handle_slash_event(state: &mut SlashState, key: KeyEvent) -> SlashAction {
    use crossterm::event::KeyCode;

    match key.code {
        KeyCode::Esc => SlashAction::Close,

        KeyCode::Enter => {
            // Clear previous result/error before dispatching.
            state.result = None;
            state.error = None;
            match parse_slash_cmd(&state.input) {
                Some(cmd) => SlashAction::Execute(cmd),
                None => {
                    if state.input.trim().is_empty() {
                        SlashAction::Close
                    } else {
                        state.error = Some(format!("Unknown command: /{}", state.input));
                        SlashAction::Continue
                    }
                }
            }
        }

        KeyCode::Tab => {
            if let Some(suggestion) = tab_complete(&state.input) {
                // Strip the leading `/` since input buffer doesn't store it.
                state.input = suggestion.trim_start_matches('/').to_string();
                state.suggestion = Some(suggestion);
            }
            SlashAction::Continue
        }

        KeyCode::Backspace => {
            state.input.pop();
            // Refresh suggestion on input change.
            state.suggestion = tab_complete(&state.input);
            // Clear previous result/error when editing.
            state.result = None;
            state.error = None;
            SlashAction::Continue
        }

        KeyCode::Char(c) => {
            state.input.push(c);
            // Refresh suggestion on input change.
            state.suggestion = tab_complete(&state.input);
            // Clear previous result/error when editing.
            state.result = None;
            state.error = None;
            SlashAction::Continue
        }

        _ => SlashAction::Continue,
    }
}

// ── draw_slash_overlay ────────────────────────────────────────────────────────

/// Draw the slash command overlay at the bottom of the screen.
///
/// Layout (bottom-aligned within `area`):
/// - Result/error line (if any) — 1 line
/// - Input line showing `/ <input>` with dimmed suggestion — 1 line
///   Total height: 2–3 lines anchored to the bottom of `area`.
pub fn draw_slash_overlay(frame: &mut ratatui::Frame, area: Rect, state: &SlashState) {
    use ratatui::style::{Color, Style};
    use ratatui::text::{Line, Span};
    use ratatui::widgets::{Clear, Paragraph};

    // Compute feedback lines (result or error), capped to avoid filling the screen.
    let max_result_lines: usize = (area.height as usize / 2).clamp(1, 10);
    let mut feedback_lines: Vec<Line> = Vec::new();

    if let Some(ref result) = state.result {
        let all_lines: Vec<&str> = result.lines().collect();
        let display_count = all_lines.len().min(max_result_lines);
        for line in &all_lines[..display_count] {
            feedback_lines.push(Line::from(Span::styled(
                format!("  {line}"),
                Style::default().fg(Color::Green),
            )));
        }
        if all_lines.len() > display_count {
            feedback_lines.push(Line::from(Span::styled(
                format!("  … ({} more lines)", all_lines.len() - display_count),
                Style::default().dim(),
            )));
        }
    } else if let Some(ref error) = state.error {
        feedback_lines.push(Line::from(Span::styled(
            format!("  {error}"),
            Style::default().fg(Color::Red),
        )));
    }

    // 2 = input line + hint line; plus feedback lines.
    let overlay_height: u16 = (2 + feedback_lines.len() as u16).min(area.height);
    let y = area.y + area.height.saturating_sub(overlay_height);
    let overlay_area = Rect::new(area.x, y, area.width, overlay_height);

    // Clear background behind overlay.
    frame.render_widget(Clear, overlay_area);

    let mut lines: Vec<Line> = Vec::new();
    lines.append(&mut feedback_lines);

    // Input line: `/ <input>` with dimmed suggestion suffix.
    let mut input_spans = vec![
        Span::styled("/ ", Style::default().bold().fg(Color::Cyan)),
        Span::raw(&state.input),
    ];
    if let Some(ref suggestion) = state.suggestion {
        let suffix = suggestion
            .trim_start_matches('/')
            .strip_prefix(state.input.as_str())
            .unwrap_or("");
        if !suffix.is_empty() {
            input_spans.push(Span::styled(suffix, Style::default().dim()));
        }
    }
    lines.push(Line::from(input_spans));

    // Hint line.
    lines.push(Line::from(Span::styled(
        "Tab complete · Enter run · Esc close",
        Style::default().dim(),
    )));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, overlay_area);
}
