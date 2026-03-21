//! Contract tests for the provider configuration Settings screen (S04).
//!
//! Run with:
//!   cargo test -p assay-tui --test settings

use std::path::PathBuf;

use assay_tui::app::{App, Screen};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────────────────

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

/// Build a minimal `.assay/` project fixture under `tmp`. Returns the project root.
fn setup_project(tmp: &TempDir) -> PathBuf {
    let root = tmp.path().to_path_buf();
    let assay_dir = root.join(".assay");
    std::fs::create_dir_all(assay_dir.join("milestones")).unwrap();
    std::fs::write(
        assay_dir.join("config.toml"),
        "project_name = \"test-project\"\n",
    )
    .unwrap();
    root
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Pressing `s` from the Dashboard opens the Settings screen.
#[test]
fn s_key_opens_settings_from_dashboard() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    assert!(
        matches!(app.screen, Screen::Dashboard),
        "should start on Dashboard"
    );
    app.handle_event(key(KeyCode::Char('s')));
    assert!(
        matches!(app.screen, Screen::Settings { .. }),
        "s key must transition to Settings screen"
    );
}

/// Pressing `Esc` from Settings returns to Dashboard without saving.
#[test]
fn esc_from_settings_returns_to_dashboard() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    app.handle_event(key(KeyCode::Char('s')));
    assert!(
        matches!(app.screen, Screen::Settings { .. }),
        "should be on Settings"
    );

    app.handle_event(key(KeyCode::Esc));
    assert!(
        matches!(app.screen, Screen::Dashboard),
        "Esc must return to Dashboard"
    );
}

/// Arrow navigation cycles through the three providers.
#[test]
fn arrow_keys_cycle_provider_selection() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    app.handle_event(key(KeyCode::Char('s')));

    // Initial selection should be 0 (Anthropic, the default).
    let initial = if let Screen::Settings { selected, .. } = &app.screen {
        *selected
    } else {
        panic!("must be on Settings screen");
    };
    assert_eq!(initial, 0, "initial selection should be 0 (Anthropic)");

    // Down once → 1 (OpenAI).
    app.handle_event(key(KeyCode::Down));
    let after_down = if let Screen::Settings { selected, .. } = &app.screen {
        *selected
    } else {
        panic!("must be on Settings screen");
    };
    assert_eq!(after_down, 1, "Down must advance selection to 1");

    // Up once → back to 0.
    app.handle_event(key(KeyCode::Up));
    let after_up = if let Screen::Settings { selected, .. } = &app.screen {
        *selected
    } else {
        panic!("must be on Settings screen");
    };
    assert_eq!(after_up, 0, "Up must return selection to 0");
}

/// Pressing `w` saves the selected provider to config.toml and returns to Dashboard.
#[test]
fn w_saves_provider_and_returns_to_dashboard() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root.clone())).unwrap();

    // Open settings, navigate to OpenAI (index 1), save.
    app.handle_event(key(KeyCode::Char('s')));
    app.handle_event(key(KeyCode::Down)); // select OpenAI
    app.handle_event(key(KeyCode::Char('w'))); // save

    assert!(
        matches!(app.screen, Screen::Dashboard),
        "w save must return to Dashboard"
    );

    // Confirm the config was actually written.
    let saved = assay_core::config::load(&root).expect("config should be loadable after save");
    let provider = saved.provider.expect("provider should be Some after save");
    assert_eq!(
        provider.provider,
        assay_types::ProviderKind::OpenAi,
        "saved provider must be OpenAI"
    );
}

/// Config survives a TUI restart: loading the App again reads the persisted provider.
#[test]
fn saved_provider_survives_restart() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);

    // First session: save Ollama (index 2).
    {
        let mut app = App::with_project_root(Some(root.clone())).unwrap();
        app.handle_event(key(KeyCode::Char('s')));
        app.handle_event(key(KeyCode::Down)); // OpenAI
        app.handle_event(key(KeyCode::Down)); // Ollama
        app.handle_event(key(KeyCode::Char('w')));
    }

    // Second session: App::with_project_root re-reads config from disk.
    let app2 = App::with_project_root(Some(root.clone())).unwrap();
    let kind = app2
        .config
        .as_ref()
        .and_then(|c| c.provider.as_ref())
        .map(|p| p.provider)
        .expect("provider should be loaded on startup");
    assert_eq!(
        kind,
        assay_types::ProviderKind::Ollama,
        "provider must be Ollama after restart"
    );
}
