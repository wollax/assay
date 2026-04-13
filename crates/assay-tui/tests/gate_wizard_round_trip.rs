//! State machine unit tests for the gate wizard.
//!
//! These tests directly drive `GateWizardState` via `handle_gate_wizard_event`
//! and verify step-through behaviour, `assemble_gate_input` output,
//! cancel semantics, and edit-mode pre-fill.
//!
//! Run with:
//!   cargo test -p assay-tui --test gate_wizard_round_trip

use assay_tui::gate_wizard::{
    GateWizardAction, GateWizardState, assemble_gate_input, handle_gate_wizard_event,
};
use assay_types::GatesSpec;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn type_str(state: &mut GateWizardState, s: &str) {
    for ch in s.chars() {
        handle_gate_wizard_event(state, key(KeyCode::Char(ch)));
    }
}

/// Drive through all 7 steps with a single criterion and no extends/includes,
/// returning the final action (should be Submit).
///
/// Sequence:
/// - Step 0: name="test-gate" + Enter
/// - Step 2: extends — press Enter to keep "(none)"
/// - Step 3: includes — press Enter to skip
/// - Step 4: criterion name="check-format", desc="verify formatting",
///   cmd="cargo fmt --check", AddAnother: 'n'
/// - Step 5: preconditions — 'n' (no)
/// - Step 6: confirm — Enter
fn drive_simple_create(state: &mut GateWizardState) -> GateWizardAction {
    // Step 0: name
    type_str(state, "test-gate");
    handle_gate_wizard_event(state, key(KeyCode::Enter));

    // Step 1: description (skip with empty)
    handle_gate_wizard_event(state, key(KeyCode::Enter));

    // Step 2: extends (keep "(none)" selected, press Enter)
    handle_gate_wizard_event(state, key(KeyCode::Enter));

    // Step 3: includes (press Enter to skip)
    handle_gate_wizard_event(state, key(KeyCode::Enter));

    // Step 4: criteria — add one criterion
    // Sub-step: Name
    type_str(state, "check-format");
    handle_gate_wizard_event(state, key(KeyCode::Enter));
    // Sub-step: Description
    type_str(state, "verify formatting");
    handle_gate_wizard_event(state, key(KeyCode::Enter));
    // Sub-step: Cmd
    type_str(state, "cargo fmt --check");
    handle_gate_wizard_event(state, key(KeyCode::Enter));
    // Sub-step: AddAnother — 'n' to decline
    handle_gate_wizard_event(state, key(KeyCode::Char('n')));

    // Step 5: preconditions — 'n' (no preconditions)
    handle_gate_wizard_event(state, key(KeyCode::Char('n')));

    // Step 6: confirm — Enter to submit
    handle_gate_wizard_event(state, key(KeyCode::Enter))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Full step-through in create mode must produce Submit with correct GateWizardInput.
#[test]
fn test_step_through_create_mode() {
    let mut state = GateWizardState::new(vec![], vec![]);
    let action = drive_simple_create(&mut state);

    match action {
        GateWizardAction::Submit(input) => {
            assert_eq!(input.slug, "test-gate");
            assert_eq!(input.description, None);
            assert_eq!(input.extends, None);
            assert!(input.include.is_empty());
            assert_eq!(input.criteria.len(), 1);
            assert_eq!(input.criteria[0].name, "check-format");
            assert_eq!(
                input.criteria[0].description,
                "verify formatting".to_string()
            );
            assert_eq!(input.criteria[0].cmd, Some("cargo fmt --check".to_string()));
            assert!(!input.overwrite, "create mode must have overwrite=false");
        }
        other => panic!("expected GateWizardAction::Submit, got {other:?}"),
    }
}

/// Esc at step 0 must produce Cancel immediately.
#[test]
fn test_cancel_at_step_0() {
    let mut state = GateWizardState::new(vec![], vec![]);
    let action = handle_gate_wizard_event(&mut state, key(KeyCode::Esc));
    assert!(
        matches!(action, GateWizardAction::Cancel),
        "Esc at step 0 must return Cancel"
    );
}

/// Esc after advancing to the criteria step must also produce Cancel.
#[test]
fn test_cancel_at_criteria_step() {
    let mut state = GateWizardState::new(vec![], vec![]);
    // Advance to step 4 (criteria)
    type_str(&mut state, "my-gate");
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter)); // step 0 → 1
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter)); // step 1 → 2
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter)); // step 2 → 3
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter)); // step 3 → 4
    assert_eq!(state.step, 4, "should be on criteria step (4)");

    let action = handle_gate_wizard_event(&mut state, key(KeyCode::Esc));
    assert!(
        matches!(action, GateWizardAction::Cancel),
        "Esc at criteria step must return Cancel"
    );
}

/// Pressing Enter with an empty name buffer at step 0 must set state.error.
#[test]
fn test_empty_name_shows_error() {
    let mut state = GateWizardState::new(vec![], vec![]);
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter));
    assert_eq!(state.step, 0, "step must stay at 0 on empty name");
    assert!(
        state.error.is_some(),
        "error must be set for empty gate name"
    );
}

/// assemble_gate_input with edit_slug=Some must produce overwrite=true.
#[test]
fn test_assemble_gate_input_overwrite_flag() {
    let mut state = GateWizardState::new(vec![], vec![]);
    // Manually set edit_slug to simulate edit mode.
    state.edit_slug = Some("existing-gate".to_string());
    // Set a name so assembly doesn't produce a blank slug.
    type_str(&mut state, "existing-gate");

    let input = assemble_gate_input(&state);
    assert!(
        input.overwrite,
        "edit mode (edit_slug=Some) must produce overwrite=true"
    );
}

/// GateWizardState::from_existing must pre-fill name, description, and selected_includes.
#[test]
fn test_from_existing_prefills_fields() {
    // Build a GatesSpec via TOML parse to avoid verbose struct construction.
    let gates: GatesSpec = toml::from_str(
        r#"
        name = "my-gate"
        description = "My gate description"

        [[criteria]]
        name = "check-lint"
        description = "run linter"
        cmd = "cargo clippy"
        "#,
    )
    .expect("parse gates spec");

    let state = GateWizardState::from_existing(&gates, "my-gate".to_string(), vec![], vec![]);

    // fields[0] should be pre-filled with the slug.
    assert_eq!(
        state.fields[0].last().map(|s| s.as_str()),
        Some("my-gate"),
        "fields[0] must be pre-filled with the gate slug"
    );
    // fields[1] should be pre-filled with the description.
    assert_eq!(
        state.fields[1].last().map(|s| s.as_str()),
        Some("My gate description"),
        "fields[1] must be pre-filled with the description"
    );
    // Criteria should be pre-filled.
    assert_eq!(state.criteria.len(), 1, "criteria must be pre-filled");
    assert_eq!(state.criteria[0].name, "check-lint");
    // edit_slug must be set.
    assert_eq!(
        state.edit_slug,
        Some("my-gate".to_string()),
        "edit_slug must be set in from_existing"
    );
}

/// Two criteria added with "add another", then decline — state.criteria has 2 entries.
#[test]
fn test_criteria_add_another_loop() {
    let mut state = GateWizardState::new(vec![], vec![]);
    // Advance to step 4 (criteria)
    type_str(&mut state, "multi-gate");
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter)); // step 0 → 1
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter)); // step 1 → 2
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter)); // step 2 → 3
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter)); // step 3 → 4

    // First criterion
    type_str(&mut state, "criterion-one");
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter)); // name
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter)); // desc (empty)
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter)); // cmd (empty)
    // AddAnother: 'y'
    handle_gate_wizard_event(&mut state, key(KeyCode::Char('y')));

    // Second criterion
    type_str(&mut state, "criterion-two");
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter)); // name
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter)); // desc (empty)
    handle_gate_wizard_event(&mut state, key(KeyCode::Enter)); // cmd (empty)
    // AddAnother: 'n'
    handle_gate_wizard_event(&mut state, key(KeyCode::Char('n')));

    assert_eq!(
        state.criteria.len(),
        2,
        "two criteria must be accumulated after add-another loop"
    );
    assert_eq!(state.criteria[0].name, "criterion-one");
    assert_eq!(state.criteria[1].name, "criterion-two");
}
