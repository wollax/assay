/// Multi-step wizard state for in-TUI milestone authoring.
pub struct WizardState {
    pub step: usize,
    pub fields: Vec<Vec<String>>,
    pub cursor: usize,
    pub chunk_count: usize,
    pub error: Option<String>,
}

/// The kind of input expected at the current wizard step.
pub enum StepKind {
    Name,
    Description,
    ChunkCount,
    ChunkName(usize),
    Criteria(usize),
}

/// The result of processing a key event in the wizard.
pub enum WizardAction {
    Continue,
    Submit(assay_core::wizard::WizardInputs),
    Cancel,
}

impl WizardState {
    pub fn new() -> Self {
        Self {
            step: 0,
            chunk_count: 0,
            cursor: 0,
            error: None,
            // Three initial slots: name (0), description (1), chunk-count input (2)
            fields: vec![
                vec![String::new()],
                vec![String::new()],
                vec![String::new()],
            ],
        }
    }

    /// Map a raw step index to a semantic `StepKind`.
    pub fn current_step_kind(&self) -> StepKind {
        let n = self.chunk_count;
        match self.step {
            0 => StepKind::Name,
            1 => StepKind::Description,
            2 => StepKind::ChunkCount,
            s if s < 3 + n => StepKind::ChunkName(s - 3),
            s if s < 3 + 2 * n => StepKind::Criteria(s - 3 - n),
            // Defensive fallback — should not occur in normal flow
            _ => StepKind::Name,
        }
    }
}

impl Default for WizardState {
    fn default() -> Self {
        Self::new()
    }
}

/// Drive the wizard forward with a single key event.
///
/// Returns [`WizardAction::Submit`] when the user completes all steps,
/// [`WizardAction::Cancel`] on Escape, and [`WizardAction::Continue`] otherwise.
pub fn handle_wizard_event(
    state: &mut WizardState,
    event: crossterm::event::KeyEvent,
) -> WizardAction {
    use crossterm::event::{KeyCode, KeyEventKind};

    // Guard: only handle Press events
    if event.kind != KeyEventKind::Press {
        return WizardAction::Continue;
    }

    // Clear error on any press
    state.error = None;

    let step = state.step;
    let kind = state.current_step_kind();

    match event.code {
        KeyCode::Esc => WizardAction::Cancel,

        KeyCode::Char(c) => {
            match kind {
                StepKind::ChunkCount => {
                    // Only accept digits 1–7; replace the entire field
                    if c.is_ascii_digit() && c >= '1' && c <= '7' {
                        state.fields[2] = vec![c.to_string()];
                        state.cursor = 1;
                    }
                    // else: silently ignore
                }
                _ => {
                    // Append character to the active buffer
                    if let Some(buf) = state.fields[step].last_mut() {
                        buf.push(c);
                        state.cursor += 1;
                    }
                }
            }
            WizardAction::Continue
        }

        KeyCode::Backspace => {
            match kind {
                StepKind::Criteria(_) => {
                    let last = state.fields[step].last().map(|s| s.len()).unwrap_or(0);
                    if last > 0 {
                        // Pop last char from active buffer
                        if let Some(buf) = state.fields[step].last_mut() {
                            buf.pop();
                            state.cursor = state.cursor.saturating_sub(1);
                        }
                    } else if state.fields[step].len() > 1 {
                        // Remove the trailing empty entry, go back to previous criterion
                        state.fields[step].pop();
                        let new_len = state.fields[step].last().map(|s| s.len()).unwrap_or(0);
                        state.cursor = new_len;
                    } else {
                        // Only one empty entry — go back a step
                        if step > 0 {
                            state.step -= 1;
                            let new_len = state.fields[state.step]
                                .last()
                                .map(|s| s.len())
                                .unwrap_or(0);
                            state.cursor = new_len;
                        }
                    }
                }
                _ => {
                    // Single-line steps: Name, Description, ChunkCount, ChunkName
                    let buf_len = state.fields[step].last().map(|s| s.len()).unwrap_or(0);
                    if buf_len > 0 {
                        if let Some(buf) = state.fields[step].last_mut() {
                            buf.pop();
                            state.cursor = state.cursor.saturating_sub(1);
                        }
                    } else if step > 0 {
                        // Empty buffer — go back a step
                        state.step -= 1;
                        let new_len = state.fields[state.step]
                            .last()
                            .map(|s| s.len())
                            .unwrap_or(0);
                        state.cursor = new_len;
                    }
                }
            }
            WizardAction::Continue
        }

        KeyCode::Enter => {
            match kind {
                StepKind::Name => {
                    let name = state.fields[step]
                        .last()
                        .map(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    if name.is_empty() {
                        state.error = Some("Name cannot be empty".to_string());
                        return WizardAction::Continue;
                    }
                    state.step += 1;
                    // Ensure next step has an active buffer
                    if state.fields[state.step]
                        .last()
                        .map(|s| s.is_empty())
                        .unwrap_or(true)
                    {
                        // already initialized in new()
                    }
                    state.cursor = state.fields[state.step]
                        .last()
                        .map(|s| s.len())
                        .unwrap_or(0);
                }

                StepKind::Description => {
                    state.step += 1;
                    state.cursor = state.fields[state.step]
                        .last()
                        .map(|s| s.len())
                        .unwrap_or(0);
                }

                StepKind::ChunkCount => {
                    let buf = state.fields[step].last().cloned().unwrap_or_default();
                    let valid = buf.len() == 1
                        && buf
                            .chars()
                            .next()
                            .map(|c| c.is_ascii_digit() && c >= '1' && c <= '7')
                            .unwrap_or(false);
                    if !valid {
                        state.error = Some("Enter a number from 1 to 7".to_string());
                        return WizardAction::Continue;
                    }
                    let n: usize = buf.parse().unwrap();
                    state.chunk_count = n;
                    // Allocate N ChunkName vecs + N Criteria vecs
                    for _ in 0..n {
                        state.fields.push(vec![String::new()]);
                    }
                    for _ in 0..n {
                        state.fields.push(vec![String::new()]);
                    }
                    state.step += 1;
                    state.cursor = 0;
                }

                StepKind::ChunkName(_) => {
                    state.step += 1;
                    state.cursor = state.fields[state.step]
                        .last()
                        .map(|s| s.len())
                        .unwrap_or(0);
                }

                StepKind::Criteria(n) => {
                    let active_empty = state.fields[step]
                        .last()
                        .map(|s| s.is_empty())
                        .unwrap_or(true);
                    if !active_empty {
                        // Start a new criterion line
                        state.fields[step].push(String::new());
                        state.cursor = 0;
                    } else {
                        // Blank Enter — criteria step done
                        let chunk_count = state.chunk_count;
                        if n + 1 < chunk_count {
                            // More criteria steps to go
                            state.step += 1;
                            state.cursor = state.fields[state.step]
                                .last()
                                .map(|s| s.len())
                                .unwrap_or(0);
                        } else {
                            // Last criteria step — assemble and submit
                            return assemble_submit(state);
                        }
                    }
                }
            }
            WizardAction::Continue
        }

        _ => WizardAction::Continue,
    }
}

fn assemble_submit(state: &WizardState) -> WizardAction {
    use assay_core::wizard::{WizardChunkInput, WizardInputs, slugify};

    let name = state.fields[0][0].clone();
    let slug = slugify(&name);
    let description = {
        let d = &state.fields[1][0];
        if d.is_empty() { None } else { Some(d.clone()) }
    };
    let n = state.chunk_count;
    let chunks = (0..n)
        .map(|i| {
            let chunk_name = state.fields[3 + i][0].clone();
            let chunk_slug = slugify(&chunk_name);
            let criteria = state.fields[3 + n + i]
                .iter()
                .filter(|s| !s.is_empty())
                .cloned()
                .collect();
            WizardChunkInput {
                slug: chunk_slug,
                name: chunk_name,
                criteria,
            }
        })
        .collect();

    WizardAction::Submit(WizardInputs {
        slug,
        name,
        description,
        chunks,
    })
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            kind: KeyEventKind::Press,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        }
    }

    fn type_str(state: &mut WizardState, s: &str) {
        for c in s.chars() {
            handle_wizard_event(state, key(KeyCode::Char(c)));
        }
    }

    /// Advance through steps 0–2 to get N chunk name/criteria slots allocated.
    fn advance_to_chunk_names(state: &mut WizardState, name: &str, n: char) {
        type_str(state, name);
        handle_wizard_event(state, key(KeyCode::Enter)); // step 0 → 1
        handle_wizard_event(state, key(KeyCode::Enter)); // step 1 → 2 (blank description)
        handle_wizard_event(state, key(KeyCode::Char(n))); // set chunk count
        handle_wizard_event(state, key(KeyCode::Enter)); // step 2 → 3
    }

    // ── step_kind tests ───────────────────────────────────────────────────────

    #[test]
    fn wizard_step_kind_n1() {
        let mut s = WizardState::new();
        s.chunk_count = 1;
        // Steps: 0=Name 1=Desc 2=ChunkCount 3=ChunkName(0) 4=Criteria(0) — total 5
        let expected: &[(usize, &str)] = &[
            (0, "Name"),
            (1, "Description"),
            (2, "ChunkCount"),
            (3, "ChunkName(0)"),
            (4, "Criteria(0)"),
        ];
        for (step, label) in expected {
            s.step = *step;
            let got = match s.current_step_kind() {
                StepKind::Name => "Name",
                StepKind::Description => "Description",
                StepKind::ChunkCount => "ChunkCount",
                StepKind::ChunkName(0) => "ChunkName(0)",
                StepKind::Criteria(0) => "Criteria(0)",
                _ => "other",
            };
            assert_eq!(got, *label, "step {step}");
        }
    }

    #[test]
    fn wizard_step_kind_n2() {
        let mut s = WizardState::new();
        s.chunk_count = 2;
        // Steps: 0=Name 1=Desc 2=CC 3=CN(0) 4=CN(1) 5=Crit(0) 6=Crit(1) — total 7
        let expected: &[(usize, &str)] = &[
            (0, "Name"),
            (1, "Desc"),
            (2, "CC"),
            (3, "CN0"),
            (4, "CN1"),
            (5, "C0"),
            (6, "C1"),
        ];
        let map = |s: &mut WizardState| match s.current_step_kind() {
            StepKind::Name => "Name",
            StepKind::Description => "Desc",
            StepKind::ChunkCount => "CC",
            StepKind::ChunkName(0) => "CN0",
            StepKind::ChunkName(1) => "CN1",
            StepKind::Criteria(0) => "C0",
            StepKind::Criteria(1) => "C1",
            _ => "other",
        };
        for (step, label) in expected {
            s.step = *step;
            assert_eq!(map(&mut s), *label, "step {step}");
        }
    }

    #[test]
    fn wizard_step_kind_n3() {
        let mut s = WizardState::new();
        s.chunk_count = 3;
        // Boundary indices: 3+3-1=5 last ChunkName, 3+3=6 first Criteria, 3+6-1=8 last Criteria
        s.step = 5;
        assert!(matches!(s.current_step_kind(), StepKind::ChunkName(2)));
        s.step = 6;
        assert!(matches!(s.current_step_kind(), StepKind::Criteria(0)));
        s.step = 8;
        assert!(matches!(s.current_step_kind(), StepKind::Criteria(2)));
    }

    // ── backspace tests ───────────────────────────────────────────────────────

    #[test]
    fn wizard_backspace_on_empty_goes_back() {
        let mut state = WizardState::new();
        state.step = 1;
        state.fields[1] = vec![String::new()]; // empty description
        handle_wizard_event(&mut state, key(KeyCode::Backspace));
        assert_eq!(state.step, 0, "should go back to step 0");
    }

    #[test]
    fn wizard_backspace_removes_last_char() {
        let mut state = WizardState::new();
        type_str(&mut state, "hello");
        assert_eq!(state.fields[0][0], "hello");
        handle_wizard_event(&mut state, key(KeyCode::Backspace));
        assert_eq!(state.fields[0][0], "hell");
        assert_eq!(state.cursor, 4);
    }

    #[test]
    fn wizard_backspace_on_criteria_removes_entry() {
        let mut state = WizardState::new();
        advance_to_chunk_names(&mut state, "M", '1');
        // Now at step 3 (ChunkName(0)); advance to criteria
        type_str(&mut state, "Chunk");
        handle_wizard_event(&mut state, key(KeyCode::Enter)); // → step 4 = Criteria(0)

        // Add one criterion, then blank Enter to get empty second entry
        type_str(&mut state, "criterion one");
        handle_wizard_event(&mut state, key(KeyCode::Enter)); // push new empty entry

        assert_eq!(state.fields[state.step].len(), 2);

        // Backspace on empty trailing entry should remove it
        handle_wizard_event(&mut state, key(KeyCode::Backspace));
        assert_eq!(state.fields[state.step].len(), 1);
        assert_eq!(state.fields[state.step][0], "criterion one");
    }

    // ── criteria blank Enter tests ─────────────────────────────────────────────

    #[test]
    fn wizard_criteria_blank_enter_advances_when_not_last() {
        // N=2: step 5=Criteria(0), step 6=Criteria(1)
        let mut state = WizardState::new();
        advance_to_chunk_names(&mut state, "M", '2');

        // step 3: chunk name 0
        type_str(&mut state, "Alpha");
        handle_wizard_event(&mut state, key(KeyCode::Enter));

        // step 4: chunk name 1
        type_str(&mut state, "Beta");
        handle_wizard_event(&mut state, key(KeyCode::Enter));

        // step 5: Criteria(0) — add criterion then blank Enter to advance
        type_str(&mut state, "crit a");
        handle_wizard_event(&mut state, key(KeyCode::Enter)); // push empty entry
        let action = handle_wizard_event(&mut state, key(KeyCode::Enter)); // blank → advance

        assert!(matches!(action, WizardAction::Continue));
        assert_eq!(state.step, 6, "should advance to Criteria(1)");
    }

    // ── submit assembly test ──────────────────────────────────────────────────

    #[test]
    fn wizard_submit_assembles_inputs() {
        let mut state = WizardState::new();

        // Step 0: name
        type_str(&mut state, "My Chunk");
        handle_wizard_event(&mut state, key(KeyCode::Enter));

        // Step 1: description (blank)
        handle_wizard_event(&mut state, key(KeyCode::Enter));

        // Step 2: chunk count = 1
        handle_wizard_event(&mut state, key(KeyCode::Char('1')));
        handle_wizard_event(&mut state, key(KeyCode::Enter));

        // Step 3: chunk name
        type_str(&mut state, "Chunk A");
        handle_wizard_event(&mut state, key(KeyCode::Enter));

        // Step 4: criteria — add one, then blank Enter → Submit
        type_str(&mut state, "does the thing");
        handle_wizard_event(&mut state, key(KeyCode::Enter)); // push empty entry
        let action = handle_wizard_event(&mut state, key(KeyCode::Enter)); // blank → Submit

        let WizardAction::Submit(inputs) = action else {
            panic!("expected Submit");
        };

        assert_eq!(inputs.slug, "my-chunk");
        assert_eq!(inputs.name, "My Chunk");
        assert!(inputs.description.is_none());
        assert_eq!(inputs.chunks.len(), 1);
        assert_eq!(inputs.chunks[0].slug, "chunk-a");
        assert_eq!(inputs.chunks[0].name, "Chunk A");
        assert_eq!(inputs.chunks[0].criteria.len(), 1);
        assert_eq!(inputs.chunks[0].criteria[0], "does the thing");
    }

    // ── non-press events are ignored ──────────────────────────────────────────

    #[test]
    fn wizard_non_press_event_is_ignored() {
        let mut state = WizardState::new();
        let release = KeyEvent {
            code: KeyCode::Char('x'),
            kind: KeyEventKind::Release,
            modifiers: KeyModifiers::NONE,
            state: KeyEventState::NONE,
        };
        handle_wizard_event(&mut state, release);
        assert_eq!(
            state.fields[0][0], "",
            "Release event should not modify state"
        );
    }

    // ── validation tests ──────────────────────────────────────────────────────

    #[test]
    fn wizard_empty_name_sets_error() {
        let mut state = WizardState::new();
        // Don't type anything — press Enter on empty name
        handle_wizard_event(&mut state, key(KeyCode::Enter));
        assert!(state.error.is_some(), "should have an error for empty name");
        assert_eq!(state.step, 0, "should stay on step 0");
    }

    #[test]
    fn wizard_invalid_chunk_count_sets_error() {
        let mut state = WizardState::new();
        // Advance to step 2
        type_str(&mut state, "My Milestone");
        handle_wizard_event(&mut state, key(KeyCode::Enter));
        handle_wizard_event(&mut state, key(KeyCode::Enter));
        // Try to confirm with empty buffer
        handle_wizard_event(&mut state, key(KeyCode::Enter));
        assert!(state.error.is_some(), "should error on empty chunk count");
        assert_eq!(state.step, 2, "should stay on step 2");
    }

    #[test]
    fn wizard_chunk_count_ignores_non_digit() {
        let mut state = WizardState::new();
        type_str(&mut state, "M");
        handle_wizard_event(&mut state, key(KeyCode::Enter));
        handle_wizard_event(&mut state, key(KeyCode::Enter));
        // Type 'a' — should be silently ignored
        handle_wizard_event(&mut state, key(KeyCode::Char('a')));
        assert_eq!(state.fields[2][0], "", "non-digit should be ignored");
        // Type '0' — out of range, should be ignored
        handle_wizard_event(&mut state, key(KeyCode::Char('0')));
        assert_eq!(state.fields[2][0], "", "'0' should be ignored");
        // Type '8' — out of range
        handle_wizard_event(&mut state, key(KeyCode::Char('8')));
        assert_eq!(state.fields[2][0], "", "'8' should be ignored");
        // Type '3' — valid
        handle_wizard_event(&mut state, key(KeyCode::Char('3')));
        assert_eq!(state.fields[2][0], "3");
    }

    #[test]
    fn wizard_error_cleared_on_next_press() {
        let mut state = WizardState::new();
        // Trigger error by pressing Enter on empty name
        handle_wizard_event(&mut state, key(KeyCode::Enter));
        assert!(state.error.is_some());
        // Any key press should clear error
        handle_wizard_event(&mut state, key(KeyCode::Char('a')));
        assert!(state.error.is_none());
    }
}
