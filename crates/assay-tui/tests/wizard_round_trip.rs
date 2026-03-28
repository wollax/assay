//! Integration tests for the in-TUI authoring wizard round-trip:
//! state machine step-through, `WizardInputs` assembly, filesystem writes,
//! cancel, backspace-to-previous-step, validation guards, and single-chunk path.
//!
//! Run with:
//!   cargo test -p assay-tui --test wizard_round_trip

use assay_core::wizard::create_from_inputs;
use assay_tui::wizard::{WizardAction, WizardState, handle_wizard_event};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a key press event with no modifiers.
fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// Feed each character in `s` as a separate `KeyCode::Char` event.
fn type_str(state: &mut WizardState, s: &str) {
    for ch in s.chars() {
        handle_wizard_event(state, key(KeyCode::Char(ch)));
    }
}

/// Drive the wizard through a complete 2-chunk sequence and return the final
/// `WizardAction`. The sequence is:
///
/// - milestone name "Test Milestone" + Enter
/// - description "A description" + Enter
/// - chunk count "2" + Enter
/// - chunk 0 name "Alpha Chunk" + Enter
/// - chunk 0 criterion "Criterion A" + Enter
/// - chunk 0 criterion cmd "cargo test" + Enter
/// - blank Enter (end chunk 0 criteria)
/// - chunk 1 name "Beta Chunk" + Enter
/// - chunk 1 criterion "Criterion B" + Enter
/// - chunk 1 criterion cmd (blank = skip) + Enter
/// - blank Enter (end chunk 1 criteria, triggers Submit)
fn drive_two_chunk_wizard(state: &mut WizardState) -> WizardAction {
    // Step 0 — milestone name
    type_str(state, "Test Milestone");
    handle_wizard_event(state, key(KeyCode::Enter));

    // Step 1 — description
    type_str(state, "A description");
    handle_wizard_event(state, key(KeyCode::Enter));

    // Step 2 — chunk count
    handle_wizard_event(state, key(KeyCode::Char('2')));
    handle_wizard_event(state, key(KeyCode::Enter));

    // Step 3 — chunk 0 name
    type_str(state, "Alpha Chunk");
    handle_wizard_event(state, key(KeyCode::Enter));

    // Step 4 — chunk 0 criteria: one criterion with cmd, then blank Enter
    type_str(state, "Criterion A");
    handle_wizard_event(state, key(KeyCode::Enter)); // name entered → cmd sub-step
    type_str(state, "cargo test");
    handle_wizard_event(state, key(KeyCode::Enter)); // cmd entered → back to name
    handle_wizard_event(state, key(KeyCode::Enter)); // blank name → end criteria

    // Step 5 — chunk 1 name
    type_str(state, "Beta Chunk");
    handle_wizard_event(state, key(KeyCode::Enter));

    // Step 6 — chunk 1 criteria: one criterion with no cmd, then blank Enter → Submit
    type_str(state, "Criterion B");
    handle_wizard_event(state, key(KeyCode::Enter)); // name entered → cmd sub-step
    handle_wizard_event(state, key(KeyCode::Enter)); // blank cmd → skip, back to name
    handle_wizard_event(state, key(KeyCode::Enter)) // blank name → Submit
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// The state machine must advance `step` through all stages of a 2-chunk wizard
/// and produce a `Submit` action on the final blank Enter.
#[test]
fn test_wizard_state_advances_through_all_steps() {
    let mut state = WizardState::new();
    assert_eq!(state.step, 0, "wizard should start at step 0");

    let action = drive_two_chunk_wizard(&mut state);

    assert!(
        matches!(action, WizardAction::Submit(_)),
        "final blank Enter in last chunk's criteria must return WizardAction::Submit"
    );
    // The state must have advanced past the last step (7 steps for a 2-chunk wizard:
    // name, description, chunk count, chunk-0 name, chunk-0 criteria, chunk-1 name,
    // chunk-1 criteria).
    assert_eq!(
        state.step, 6,
        "step should be at the last criteria step (6 for a 2-chunk wizard)"
    );
}

/// Completing the wizard must produce a `WizardInputs` with correct slug, name,
/// description, and per-chunk slugs/names/criteria.
#[test]
fn test_wizard_submit_produces_correct_wizard_inputs() {
    let mut state = WizardState::new();
    let action = drive_two_chunk_wizard(&mut state);

    let inputs = match action {
        WizardAction::Submit(inputs) => inputs,
        other => panic!("expected WizardAction::Submit, got {other:?}"),
    };

    assert_eq!(
        inputs.slug, "test-milestone",
        "slug must be slugified from name"
    );
    assert_eq!(inputs.name, "Test Milestone");
    assert_eq!(
        inputs.description,
        Some("A description".to_string()),
        "description must be captured from step 1"
    );
    assert_eq!(inputs.chunks.len(), 2, "must have exactly 2 chunks");

    assert_eq!(inputs.chunks[0].slug, "alpha-chunk");
    assert_eq!(inputs.chunks[0].name, "Alpha Chunk");
    assert_eq!(
        inputs.chunks[0].criteria.len(),
        1,
        "chunk 0 must have exactly one criterion"
    );
    assert_eq!(inputs.chunks[0].criteria[0].name, "Criterion A");
    assert_eq!(
        inputs.chunks[0].criteria[0].cmd,
        Some("cargo test".to_string()),
        "chunk 0 criterion must have cmd 'cargo test'"
    );

    assert_eq!(inputs.chunks[1].slug, "beta-chunk");
    assert_eq!(inputs.chunks[1].name, "Beta Chunk");
    assert_eq!(
        inputs.chunks[1].criteria.len(),
        1,
        "chunk 1 must have exactly one criterion"
    );
    assert_eq!(inputs.chunks[1].criteria[0].name, "Criterion B");
    assert_eq!(
        inputs.chunks[1].criteria[0].cmd, None,
        "chunk 1 criterion must have cmd None (skipped)"
    );
}

/// Full round-trip: synthetic key events → `WizardInputs` → `create_from_inputs`
/// → milestone TOML and chunk `gates.toml` files written to tempdir.
#[test]
fn test_wizard_round_trip_writes_files() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    let specs_dir = assay_dir.join("specs");
    std::fs::create_dir_all(assay_dir.join("milestones")).unwrap();
    std::fs::create_dir_all(&specs_dir).unwrap();

    let mut state = WizardState::new();
    let action = drive_two_chunk_wizard(&mut state);

    let inputs = match action {
        WizardAction::Submit(inputs) => inputs,
        _ => panic!("expected Submit"),
    };

    let result = create_from_inputs(&inputs, &assay_dir, &specs_dir);
    assert!(
        result.is_ok(),
        "create_from_inputs must succeed: {:?}",
        result.err()
    );

    let milestone_toml = assay_dir.join("milestones").join("test-milestone.toml");
    assert!(
        milestone_toml.exists(),
        "milestone TOML must exist at {milestone_toml:?}"
    );
    assert!(
        std::fs::metadata(&milestone_toml).unwrap().len() > 0,
        "milestone TOML must be non-empty"
    );

    let alpha_gates = specs_dir.join("alpha-chunk").join("gates.toml");
    assert!(
        alpha_gates.exists(),
        "alpha-chunk gates.toml must exist at {alpha_gates:?}"
    );

    let beta_gates = specs_dir.join("beta-chunk").join("gates.toml");
    assert!(
        beta_gates.exists(),
        "beta-chunk gates.toml must exist at {beta_gates:?}"
    );
}

/// Pressing Esc at any point must produce `WizardAction::Cancel`.
#[test]
fn test_wizard_cancel_returns_cancel_action() {
    let mut state = WizardState::new();

    // Type a few characters for the milestone name, then press Esc.
    type_str(&mut state, "Some name");
    let action = handle_wizard_event(&mut state, key(KeyCode::Esc));

    assert!(
        matches!(action, WizardAction::Cancel),
        "Esc must return WizardAction::Cancel at any step"
    );
}

/// Pressing Backspace on an empty field must go back to the previous step.
#[test]
fn test_wizard_backspace_on_empty_field_goes_back() {
    let mut state = WizardState::new();

    // Complete step 0 (milestone name) to reach step 1 (description).
    type_str(&mut state, "Test Milestone");
    handle_wizard_event(&mut state, key(KeyCode::Enter));
    assert_eq!(
        state.step, 1,
        "should be on step 1 (description) after completing step 0"
    );

    // Press Backspace with an empty description buffer → should return to step 0.
    handle_wizard_event(&mut state, key(KeyCode::Backspace));
    assert_eq!(
        state.step, 0,
        "backspace on empty field must decrement step to 0, got {}",
        state.step
    );
}

// ── Validation guard tests ────────────────────────────────────────────────────

/// Pressing Enter with an empty name buffer must keep step at 0 and set an error.
#[test]
fn test_wizard_empty_name_blocks_advance() {
    let mut state = WizardState::new();
    handle_wizard_event(&mut state, key(KeyCode::Enter));
    assert_eq!(state.step, 0, "step must stay at 0 on empty name");
    assert!(state.error.is_some(), "error must be set for empty name");
}

/// Pressing Enter on the chunk-count step without typing a digit must keep step
/// at 2 and set an error.
#[test]
fn test_wizard_empty_chunk_count_blocks_advance() {
    let mut state = WizardState::new();
    type_str(&mut state, "My Milestone");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 0 → 1
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 1 → 2 (blank desc OK)
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // Enter with empty digit buffer
    assert_eq!(state.step, 2, "step must stay at 2 on empty chunk count");
    assert!(
        state.error.is_some(),
        "error must be set for empty chunk count"
    );
}

/// Pressing Enter on an empty chunk-name step must keep the step in place and
/// set an error.
#[test]
fn test_wizard_empty_chunk_name_blocks_advance() {
    let mut state = WizardState::new();
    type_str(&mut state, "My Milestone");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 0 → 1
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 1 → 2
    handle_wizard_event(&mut state, key(KeyCode::Char('1')));
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 2 → 3
    // Step 3 is chunk-0 name; press Enter without typing anything.
    handle_wizard_event(&mut state, key(KeyCode::Enter));
    assert_eq!(state.step, 3, "step must stay at 3 on empty chunk name");
    assert!(
        state.error.is_some(),
        "error must be set for empty chunk name"
    );
}

// ── Single-chunk path ─────────────────────────────────────────────────────────

/// A single-chunk wizard must emit `Submit` on the final blank Enter.
///
/// The last-chunk check is `i < chunk_count - 1`. For `chunk_count == 1` that
/// evaluates to `0 < 0 == false`, which must trigger `Submit` not a loop.
#[test]
fn test_wizard_single_chunk_submits() {
    let mut state = WizardState::new();
    type_str(&mut state, "Solo Milestone");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 0 → 1
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 1 → 2 (blank desc)
    handle_wizard_event(&mut state, key(KeyCode::Char('1')));
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 2 → 3
    type_str(&mut state, "Only Chunk");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 3 → 4
    type_str(&mut state, "Criterion X");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // name → cmd sub-step
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // blank cmd → skip, back to name
    let action = handle_wizard_event(&mut state, key(KeyCode::Enter)); // blank name → Submit
    assert!(
        matches!(action, WizardAction::Submit(_)),
        "single-chunk wizard must produce Submit on final blank Enter"
    );
}

// ── Backspace from cmd sub-step ───────────────────────────────────────────────

/// Pressing Backspace on an empty cmd line must cancel the cmd sub-step
/// (reset the flag) and return to the name sub-step within the same criteria step.
#[test]
fn test_backspace_from_cmd_sub_step_resets_flag() {
    let mut state = WizardState::new();

    // Drive to a single-chunk wizard's criteria step.
    type_str(&mut state, "BS Test");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 0 → 1
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 1 → 2 (blank desc)
    handle_wizard_event(&mut state, key(KeyCode::Char('1')));
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 2 → 3
    type_str(&mut state, "Chunk A");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 3 → 4 (criteria)

    let criteria_step = state.step;

    // Enter a criterion name → cmd sub-step
    type_str(&mut state, "Criterion A");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // name → cmd
    assert!(state.criteria_awaiting_cmd, "should be in cmd sub-step");
    assert_eq!(state.step, criteria_step, "step must stay in criteria step");

    // Backspace on empty cmd line → should cancel cmd sub-step, return to name
    handle_wizard_event(&mut state, key(KeyCode::Backspace));
    assert!(
        !state.criteria_awaiting_cmd,
        "flag must reset when backspacing out of cmd sub-step"
    );
    assert_eq!(
        state.step, criteria_step,
        "step must stay in criteria step after cmd-backspace"
    );
}

/// After backspacing out of cmd sub-step, re-entering the name and completing
/// the wizard must produce correct criteria without corruption.
#[test]
fn test_backspace_from_cmd_sub_step_then_complete() {
    let mut state = WizardState::new();

    type_str(&mut state, "BS Complete");
    handle_wizard_event(&mut state, key(KeyCode::Enter));
    handle_wizard_event(&mut state, key(KeyCode::Enter));
    handle_wizard_event(&mut state, key(KeyCode::Char('1')));
    handle_wizard_event(&mut state, key(KeyCode::Enter));
    type_str(&mut state, "Chunk A");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // → criteria step

    // Enter name, go to cmd, then backspace out
    type_str(&mut state, "Crit 1");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // name → cmd
    assert!(state.criteria_awaiting_cmd);
    handle_wizard_event(&mut state, key(KeyCode::Backspace)); // cancel cmd sub-step
    assert!(!state.criteria_awaiting_cmd);

    // Now re-enter the name flow: press Enter on restored "Crit 1" → cmd sub-step again
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // name → cmd
    assert!(state.criteria_awaiting_cmd);
    type_str(&mut state, "cargo test");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // cmd → name

    // Blank name → submit
    let action = handle_wizard_event(&mut state, key(KeyCode::Enter));
    let inputs = match action {
        WizardAction::Submit(inputs) => inputs,
        other => panic!("expected Submit, got {other:?}"),
    };

    assert_eq!(inputs.chunks[0].criteria.len(), 1);
    assert_eq!(inputs.chunks[0].criteria[0].name, "Crit 1");
    assert_eq!(
        inputs.chunks[0].criteria[0].cmd,
        Some("cargo test".to_string())
    );
}

// ── Cmd-aware criteria tests ──────────────────────────────────────────────────

/// The wizard must alternate name→cmd sub-steps and produce CriterionInput
/// structs with correct cmd values (Some for provided, None for skipped).
#[test]
fn test_wizard_criteria_cmd_round_trip() {
    let mut state = WizardState::new();

    // Drive through to a single-chunk wizard with 2 criteria:
    // criterion 1: name="Build check", cmd="cargo build"
    // criterion 2: name="Lint check", cmd=None (skipped)
    type_str(&mut state, "Cmd Test");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 0 → 1
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 1 → 2 (blank desc)
    handle_wizard_event(&mut state, key(KeyCode::Char('1')));
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 2 → 3

    type_str(&mut state, "My Chunk");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // step 3 → 4 (criteria)

    // Criterion 1: name then cmd
    type_str(&mut state, "Build check");
    assert!(!state.criteria_awaiting_cmd, "should be in name phase");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // name → cmd
    assert!(
        state.criteria_awaiting_cmd,
        "should be in cmd phase after entering name"
    );
    type_str(&mut state, "cargo build");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // cmd → back to name
    assert!(
        !state.criteria_awaiting_cmd,
        "should be back in name phase after cmd"
    );

    // Criterion 2: name then skip cmd
    type_str(&mut state, "Lint check");
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // name → cmd
    assert!(state.criteria_awaiting_cmd);
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // blank cmd → skip

    // Blank name → submit (single chunk)
    let action = handle_wizard_event(&mut state, key(KeyCode::Enter));
    let inputs = match action {
        WizardAction::Submit(inputs) => inputs,
        other => panic!("expected Submit, got {other:?}"),
    };

    assert_eq!(inputs.chunks.len(), 1);
    let criteria = &inputs.chunks[0].criteria;
    assert_eq!(criteria.len(), 2, "must have 2 criteria");

    assert_eq!(criteria[0].name, "Build check");
    assert_eq!(criteria[0].cmd, Some("cargo build".to_string()));
    assert_eq!(criteria[0].description, "");

    assert_eq!(criteria[1].name, "Lint check");
    assert_eq!(criteria[1].cmd, None);
    assert_eq!(criteria[1].description, "");
}

/// The two-chunk wizard produces correct cmd values from `drive_two_chunk_wizard`.
#[test]
fn test_wizard_two_chunk_cmd_values() {
    let mut state = WizardState::new();
    let action = drive_two_chunk_wizard(&mut state);

    let inputs = match action {
        WizardAction::Submit(inputs) => inputs,
        other => panic!("expected Submit, got {other:?}"),
    };

    // Chunk 0: "Criterion A" with cmd "cargo test"
    assert_eq!(
        inputs.chunks[0].criteria[0].cmd,
        Some("cargo test".to_string())
    );
    // Chunk 1: "Criterion B" with no cmd
    assert_eq!(inputs.chunks[1].criteria[0].cmd, None);
}
