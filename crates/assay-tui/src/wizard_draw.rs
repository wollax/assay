use assay_core::wizard::slugify;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

use crate::wizard::{StepKind, WizardState};

/// Render the wizard popup over whatever is already drawn on `frame`.
///
/// The popup is a centered 64×14 block. The cursor is positioned at the
/// active input field so the terminal hardware cursor is visible.
pub fn draw_wizard(frame: &mut Frame, state: &WizardState) {
    let area = frame.area();

    // Guard: clamp popup dimensions so Layout does not panic on narrow terminals.
    let popup_width = area.width.min(64);
    let popup_height = area.height.min(14);

    // Compute popup top-left: centered horizontally and vertically.
    let popup_x = area.x + area.width.saturating_sub(popup_width) / 2;
    let popup_y = area.y + area.height.saturating_sub(popup_height) / 2;

    let popup_area = Rect {
        x: popup_x,
        y: popup_y,
        width: popup_width,
        height: popup_height,
    };

    // Clear dashboard text behind the popup.
    frame.render_widget(Clear, popup_area);

    // Outer bordered block.
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" New Milestone ");
    let inner_area = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    // Build content lines.
    let total_steps = if state.chunk_count > 0 {
        3 + 2 * state.chunk_count
    } else {
        5 // estimated minimum (1 chunk)
    };
    let current_step_display = state.step + 1;

    let mut lines: Vec<Line> = Vec::new();

    // Line 1: step counter.
    lines.push(Line::from(Span::styled(
        format!("Step {current_step_display} of {total_steps}"),
        Style::default().dim(),
    )));

    // Line 2: prompt for the current step.
    let prompt = step_prompt(state);
    lines.push(Line::from(prompt));

    // Accumulated criteria (for Criteria steps).
    let accumulated_count = match state.current_step_kind() {
        StepKind::Criteria(_) => {
            let step = state.step;
            let acc = state.fields[step].len().saturating_sub(1);
            for criterion in state.fields[step].iter().take(acc) {
                lines.push(Line::from(Span::styled(
                    format!("  • {criterion}"),
                    Style::default().dim(),
                )));
            }
            acc
        }
        _ => 0,
    };

    // Active input buffer.
    let active_buf = state
        .fields
        .get(state.step)
        .and_then(|v| v.last())
        .map(|s| s.as_str())
        .unwrap_or("");
    lines.push(Line::from(format!("> {active_buf}")));

    // Slug hint: shown for Name and ChunkName steps when buffer is non-empty.
    let show_slug_hint = matches!(
        state.current_step_kind(),
        StepKind::Name | StepKind::ChunkName(_)
    ) && !active_buf.is_empty();

    if show_slug_hint {
        let slug = slugify(active_buf);
        lines.push(Line::from(Span::styled(
            format!("  → {slug}"),
            Style::default().dim(),
        )));
    }

    // Error line.
    if let Some(ref err) = state.error {
        lines.push(Line::from(Span::styled(
            err.clone(),
            Style::default().fg(Color::Red),
        )));
    }

    // Key hints (always last visible line).
    lines.push(Line::from(Span::styled(
        "[Enter] confirm  [Esc] cancel  [Backspace] back",
        Style::default().dim(),
    )));

    frame.render_widget(Paragraph::new(lines), inner_area);

    // Position the hardware cursor at the active input field.
    // Row: top of inner area + 2 (step counter + prompt) + accumulated criteria.
    let cursor_row = inner_area.y + 2 + accumulated_count as u16;
    // Col: left of inner area + "> ".len() (2) + cursor position.
    let cursor_col = inner_area.x + 2 + state.cursor as u16;

    // Guard: only set cursor if it falls within the popup.
    if cursor_row < inner_area.y + inner_area.height && cursor_col < inner_area.x + inner_area.width
    {
        frame.set_cursor_position((cursor_col, cursor_row));
    }
}

/// Returns the prompt string for the current wizard step.
fn step_prompt(state: &WizardState) -> String {
    match state.current_step_kind() {
        StepKind::Name => "Milestone name:".to_string(),
        StepKind::Description => "Description (optional):".to_string(),
        StepKind::ChunkCount => "Number of chunks (1–7):".to_string(),
        StepKind::ChunkName(i) => format!("Chunk {} name:", i + 1),
        StepKind::Criteria(i) => {
            format!(
                "Chunk {} criteria — one per line, blank line to finish:",
                i + 1
            )
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

// `draw_wizard` is a rendering function verified by visual UAT.
// We only test the pure helper here.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_prompt_returns_expected_strings() {
        let mut s = WizardState::new();
        s.chunk_count = 2;

        let prompts: Vec<(usize, &str)> = vec![
            (0, "Milestone name:"),
            (1, "Description (optional):"),
            (2, "Number of chunks (1–7):"),
        ];
        for (step, expected) in prompts {
            s.step = step;
            assert_eq!(step_prompt(&s), expected, "step {step}");
        }

        s.step = 3;
        assert!(step_prompt(&s).contains("Chunk 1 name"));
        s.step = 4;
        assert!(step_prompt(&s).contains("Chunk 2 name"));
        s.step = 5;
        assert!(step_prompt(&s).contains("Chunk 1 criteria"));
        s.step = 6;
        assert!(step_prompt(&s).contains("Chunk 2 criteria"));
    }
}
