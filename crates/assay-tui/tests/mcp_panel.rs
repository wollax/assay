//! Contract tests for the MCP server configuration panel (S04).
//!
//! Run with:
//!   cargo test -p assay-tui --test mcp_panel

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

/// Pressing `m` from the Dashboard opens the MCP panel with empty servers
/// when no `mcp.json` exists.
#[test]
fn mcp_panel_loads_empty_when_no_file() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root)).unwrap();

    assert!(
        matches!(app.screen, Screen::Dashboard),
        "should start on Dashboard"
    );

    app.handle_event(key(KeyCode::Char('m')));

    match &app.screen {
        Screen::McpPanel {
            servers,
            selected,
            add_form,
            error,
        } => {
            assert!(servers.is_empty(), "servers must be empty when no mcp.json");
            assert_eq!(*selected, 0);
            assert!(add_form.is_none());
            assert!(error.is_none());
        }
        other => panic!("expected McpPanel, got {:?}", screen_name(other)),
    }
}

/// Pressing `m` from the Dashboard loads servers from an existing `mcp.json`.
#[test]
fn mcp_panel_loads_from_mcp_json() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);

    // Write a valid mcp.json fixture.
    let mcp_json = r#"{
        "mcpServers": {
            "beta-server": { "command": "npx", "args": ["-y", "beta"] },
            "alpha-server": { "command": "node", "args": [] }
        }
    }"#;
    std::fs::write(root.join(".assay").join("mcp.json"), mcp_json).unwrap();

    let mut app = App::with_project_root(Some(root)).unwrap();
    app.handle_event(key(KeyCode::Char('m')));

    match &app.screen {
        Screen::McpPanel { servers, .. } => {
            assert_eq!(servers.len(), 2, "should load 2 servers");
            // Entries must be sorted alphabetically.
            assert_eq!(servers[0].name, "alpha-server");
            assert_eq!(servers[0].command, "node");
            assert_eq!(servers[1].name, "beta-server");
            assert_eq!(servers[1].command, "npx");
            assert_eq!(servers[1].args, vec!["-y", "beta"]);
        }
        other => panic!("expected McpPanel, got {:?}", screen_name(other)),
    }
}

/// Pressing `m`, then `a`, typing name + Tab + command + Enter, then `w`
/// writes a new server to `mcp.json` on disk.
#[test]
fn mcp_panel_add_server_writes_file() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);
    let mut app = App::with_project_root(Some(root.clone())).unwrap();

    // Open MCP panel.
    app.handle_event(key(KeyCode::Char('m')));
    // Open add-server form.
    app.handle_event(key(KeyCode::Char('a')));
    // Type server name.
    for c in "my-server".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    // Tab to command field.
    app.handle_event(key(KeyCode::Tab));
    // Type command.
    for c in "npx -y my-pkg".chars() {
        app.handle_event(key(KeyCode::Char(c)));
    }
    // Submit form.
    app.handle_event(key(KeyCode::Enter));
    // Write to disk.
    app.handle_event(key(KeyCode::Char('w')));

    // Verify file on disk.
    let content = std::fs::read_to_string(root.join(".assay").join("mcp.json"))
        .expect("mcp.json should exist after write");
    assert!(
        content.contains("my-server"),
        "mcp.json must contain the new server name"
    );
}

/// Start with one server in `mcp.json`; press `m`, then `d`, then `w`.
/// Assert `mcp.json` has empty `mcpServers`.
#[test]
fn mcp_panel_delete_server_writes_file() {
    let tmp = TempDir::new().unwrap();
    let root = setup_project(&tmp);

    // Pre-populate mcp.json with one server.
    let mcp_json = r#"{ "mcpServers": { "doomed": { "command": "rm", "args": [] } } }"#;
    std::fs::write(root.join(".assay").join("mcp.json"), mcp_json).unwrap();

    let mut app = App::with_project_root(Some(root.clone())).unwrap();

    // Open MCP panel.
    app.handle_event(key(KeyCode::Char('m')));
    // Delete selected server.
    app.handle_event(key(KeyCode::Char('d')));
    // Write to disk.
    app.handle_event(key(KeyCode::Char('w')));

    // Verify file on disk.
    let content = std::fs::read_to_string(root.join(".assay").join("mcp.json"))
        .expect("mcp.json should exist after write");
    // Parse and check mcpServers is empty.
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    let servers = parsed["mcpServers"].as_object().unwrap();
    assert!(
        servers.is_empty(),
        "mcpServers must be empty after deleting the only server"
    );
}

// ── Debug helper ──────────────────────────────────────────────────────────────

fn screen_name(screen: &Screen) -> &'static str {
    match screen {
        Screen::NoProject => "NoProject",
        Screen::Dashboard => "Dashboard",
        Screen::Wizard(_) => "Wizard",
        Screen::LoadError(_) => "LoadError",
        Screen::MilestoneDetail { .. } => "MilestoneDetail",
        Screen::ChunkDetail { .. } => "ChunkDetail",
        Screen::Settings { .. } => "Settings",
        Screen::AgentRun { .. } => "AgentRun",
        Screen::McpPanel { .. } => "McpPanel",
        Screen::Analytics => "Analytics",
        Screen::TraceViewer { .. } => "TraceViewer",
        Screen::GateWizard(_) => "GateWizard",
    }
}
