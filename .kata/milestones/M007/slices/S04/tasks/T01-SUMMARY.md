---
id: T01
parent: S04
milestone: M007
provides:
  - McpServerEntry struct with Serialize/Deserialize/Clone/Debug
  - AddServerForm struct for inline add-server UI state
  - mcp_config_load — reads .assay/mcp.json, returns sorted entries or empty vec
  - mcp_config_save — atomic JSON write via NamedTempFile (D093 pattern)
  - Screen::McpPanel variant with servers, selected, add_form, error fields
key_files:
  - crates/assay-tui/src/mcp_panel.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/tests/mcp_panel.rs
key_decisions:
  - "McpConfigFile uses HashMap<String, McpServerValue> for serde, matching the { mcpServers: { name: { command, args } } } JSON shape"
  - "mcp_config_load returns empty vec (not error) on missing/invalid file — graceful degradation"
patterns_established:
  - "mcp_panel module follows the same atomic-write pattern as config::save (NamedTempFile → sync → persist)"
  - "Integration tests use setup_project + key helpers matching settings.rs pattern"
observability_surfaces:
  - "Screen::McpPanel.error field surfaces I/O errors inline"
  - "mcp_config_save returns Err(String) with descriptive messages on failure"
duration: 12min
verification_result: passed
completed_at: 2026-03-23T12:00:00Z
blocker_discovered: false
---

# T01: MCP panel types, JSON I/O, Screen variant, and integration tests

**Added McpServerEntry/AddServerForm types, atomic mcp.json I/O functions, Screen::McpPanel variant, and 4 integration tests (2 pass now, 2 await T02 event wiring)**

## What Happened

Created `mcp_panel.rs` with the data model (`McpServerEntry`, `AddServerForm`), internal serde structs (`McpConfigFile`, `McpServerValue`), and two I/O functions (`mcp_config_load`, `mcp_config_save`). The save function uses the NamedTempFile-write-sync-persist pattern from D093. The load function returns an empty vec on missing/invalid files for graceful degradation, and sorts entries alphabetically by name.

Added `Screen::McpPanel` variant to the `Screen` enum with `servers`, `selected`, `add_form`, and `error` fields. Wired the `m` key on the Dashboard to load servers and transition to the MCP panel. Added placeholder draw/event arms — T02 will implement full rendering and event handling.

Wrote 4 integration tests in `tests/mcp_panel.rs` plus 2 unit tests in the module itself for JSON round-tripping.

## Verification

- `cargo build -p assay-tui` — compiles with no errors ✓
- `cargo clippy -p assay-tui` — no new warnings (only pre-existing warnings in other files) ✓
- `cargo test -p assay-tui mcp_panel::tests` — 2/2 unit tests pass (round_trip_json, load_missing_file_returns_empty) ✓
- `cargo test -p assay-tui --test mcp_panel` — 2/4 integration tests pass (load empty, load from file); 2/4 fail as expected (add/delete need T02 event wiring) ✓

### Slice-level checks
- `cargo test -p assay-tui --test mcp_panel` — 2/4 pass (partial, expected for T01)
- `cargo test --workspace` — not run (deferred to final task)
- `just ready` — not run (deferred to final task)

## Diagnostics

- `mcp_config_load(root)` can be called from tests or future code to inspect `.assay/mcp.json` state
- `Screen::McpPanel.error` field surfaces load/save failures inline for the user
- `mcp_config_save` returns `Err(String)` with descriptive context on any I/O failure

## Deviations

None.

## Known Issues

- Integration tests `mcp_panel_add_server_writes_file` and `mcp_panel_delete_server_writes_file` fail because `a`, `d`, `w` event handlers in `Screen::McpPanel` are not yet implemented — this is T02's scope.

## Files Created/Modified

- `crates/assay-tui/Cargo.toml` — added serde + serde_json workspace deps
- `crates/assay-tui/src/mcp_panel.rs` — new module with types and I/O functions
- `crates/assay-tui/src/lib.rs` — added `pub mod mcp_panel`
- `crates/assay-tui/src/app.rs` — added Screen::McpPanel variant, `m` key handler, placeholder draw/event arms
- `crates/assay-tui/tests/mcp_panel.rs` — 4 integration test functions
