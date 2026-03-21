use assay_core::wizard::create_from_inputs;
use assay_tui::wizard::{WizardAction, WizardState, handle_wizard_event};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tempfile::TempDir;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        kind: KeyEventKind::Press,
        modifiers: KeyModifiers::NONE,
        state: crossterm::event::KeyEventState::NONE,
    }
}

fn type_str(state: &mut WizardState, s: &str) {
    for c in s.chars() {
        handle_wizard_event(state, key(KeyCode::Char(c)));
    }
}

#[test]
fn wizard_round_trip() {
    let tmp = TempDir::new().unwrap();
    let assay_dir = tmp.path().join(".assay");
    let specs_dir = assay_dir.join("specs");
    let mut state = WizardState::new();

    // step 0: milestone name
    type_str(&mut state, "Auth Layer");
    handle_wizard_event(&mut state, key(KeyCode::Enter));

    // step 1: description (blank — skip)
    handle_wizard_event(&mut state, key(KeyCode::Enter));

    // step 2: chunk count = 2
    handle_wizard_event(&mut state, key(KeyCode::Char('2')));
    handle_wizard_event(&mut state, key(KeyCode::Enter));

    // step 3: chunk name 1
    type_str(&mut state, "Login");
    handle_wizard_event(&mut state, key(KeyCode::Enter));

    // step 4: chunk name 2
    type_str(&mut state, "Register");
    handle_wizard_event(&mut state, key(KeyCode::Enter));

    // step 5: criteria for chunk 1
    type_str(&mut state, "User can log in with valid credentials");
    handle_wizard_event(&mut state, key(KeyCode::Enter));
    handle_wizard_event(&mut state, key(KeyCode::Enter)); // blank = done

    // step 6: criteria for chunk 2 → Submit
    type_str(&mut state, "User can create an account");
    handle_wizard_event(&mut state, key(KeyCode::Enter));
    let action = handle_wizard_event(&mut state, key(KeyCode::Enter)); // blank = Submit

    let WizardAction::Submit(inputs) = action else {
        panic!("expected Submit, got Continue or Cancel");
    };

    let result = create_from_inputs(&inputs, &assay_dir, &specs_dir);
    assert!(
        result.is_ok(),
        "create_from_inputs failed: {:?}",
        result.err()
    );
    assert!(
        assay_dir.join("milestones/auth-layer.toml").exists(),
        "milestone TOML missing"
    );
    assert!(
        specs_dir.join("login/gates.toml").exists(),
        "login gates.toml missing"
    );
    assert!(
        specs_dir.join("register/gates.toml").exists(),
        "register gates.toml missing"
    );
}
