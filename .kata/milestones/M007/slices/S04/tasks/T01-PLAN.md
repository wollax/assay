---
estimated_steps: 5
estimated_files: 5
---

# T01: MCP panel types, JSON I/O, Screen variant, and integration tests

**Slice:** S04 — MCP Server Configuration Panel
**Milestone:** M007

## Description

Establish the data model (`McpServerEntry`, `AddServerForm`), JSON file I/O functions (`mcp_config_load`, `mcp_config_save`), add the `Screen::McpPanel` variant to `app.rs`, and write all 4 integration tests. The tests define the acceptance criteria for the slice — some will initially fail because event handling isn't wired yet (that's T02). The I/O functions are independently testable and should pass.

## Steps

1. Add `serde = { workspace = true }` and `serde_json = { workspace = true }` to `crates/assay-tui/Cargo.toml` `[dependencies]` section
2. Create `crates/assay-tui/src/mcp_panel.rs` with:
   - `McpServerEntry { name: String, command: String, args: Vec<String> }` with `#[derive(Serialize, Deserialize, Clone, Debug)]`
   - `AddServerForm { name: String, command: String, active_field: usize }` (active_field: 0=name, 1=command)
   - `McpConfigFile` internal struct for JSON serde: `{ mcpServers: HashMap<String, McpServerValue> }` where `McpServerValue { command: String, args: Vec<String> }`
   - `mcp_config_load(root: &Path) -> Vec<McpServerEntry>` — reads `.assay/mcp.json`, returns empty vec if missing, parses and sorts alphabetically by name
   - `mcp_config_save(root: &Path, servers: &[McpServerEntry]) -> Result<(), String>` — serializes to `{ "mcpServers": { ... } }` JSON, writes atomically via NamedTempFile (D093 pattern from `config::save`)
3. Add `pub mod mcp_panel;` to `crates/assay-tui/src/lib.rs`
4. Add `Screen::McpPanel { servers: Vec<McpServerEntry>, selected: usize, add_form: Option<AddServerForm>, error: Option<String> }` variant to the `Screen` enum in `app.rs`. Import `McpServerEntry` and `AddServerForm` from `crate::mcp_panel` (or `assay_tui::mcp_panel` as needed)
5. Write `crates/assay-tui/tests/mcp_panel.rs` with 4 tests:
   - `mcp_panel_loads_empty_when_no_file` — `App::with_project_root` on a tempdir without `mcp.json`; press `m`; assert `Screen::McpPanel` with empty servers vec
   - `mcp_panel_loads_from_mcp_json` — write a valid `mcp.json` to tempdir fixture; press `m`; assert `Screen::McpPanel` with correct servers
   - `mcp_panel_add_server_writes_file` — press `m`, then `a`, type name + Tab + command + Enter, then `w`; assert `mcp.json` on disk contains the new server
   - `mcp_panel_delete_server_writes_file` — start with one server in `mcp.json`; press `m`, then `d`, then `w`; assert `mcp.json` has empty `mcpServers`
   - Use `setup_project` + `key` helpers matching the pattern from `tests/settings.rs`

## Must-Haves

- [ ] `McpServerEntry` struct with Serialize, Deserialize, Clone, Debug derives — round-trips through JSON
- [ ] `mcp_config_load` returns empty vec for missing file, parsed sorted vec for valid JSON
- [ ] `mcp_config_save` writes atomic JSON using NamedTempFile pattern
- [ ] `Screen::McpPanel` variant exists in `Screen` enum with servers, selected, add_form, error fields
- [ ] 4 integration test functions compile in `tests/mcp_panel.rs`
- [ ] `cargo build -p assay-tui` succeeds with no errors

## Verification

- `cargo build -p assay-tui` — compiles without errors
- `cargo clippy -p assay-tui` — no warnings
- `cargo test -p assay-tui --test mcp_panel` — compiles (tests that depend on event handling may fail, but load/save tests should pass)
- Manually verify `McpServerEntry` JSON round-trip by inspecting `mcp_config_load` + `mcp_config_save` test output

## Observability Impact

- Signals added/changed: `error: Option<String>` field on `Screen::McpPanel` variant for surfacing I/O errors
- How a future agent inspects this: Read `Screen::McpPanel.error` in tests; read `.assay/mcp.json` on disk
- Failure state exposed: `mcp_config_load` returns empty vec on missing file (not error); `mcp_config_save` returns `Err(String)` with descriptive message on I/O failure

## Inputs

- `crates/assay-tui/src/app.rs` — existing Screen enum, App struct, D089/D097/D105 patterns
- `crates/assay-tui/tests/settings.rs` — `setup_project` + `key` helper pattern
- `crates/assay-core/src/config/mod.rs` — NamedTempFile atomic write pattern (D093)

## Expected Output

- `crates/assay-tui/Cargo.toml` — with serde + serde_json deps added
- `crates/assay-tui/src/mcp_panel.rs` — new module with types and I/O functions
- `crates/assay-tui/src/lib.rs` — `pub mod mcp_panel` added
- `crates/assay-tui/src/app.rs` — `Screen::McpPanel` variant added
- `crates/assay-tui/tests/mcp_panel.rs` — 4 test functions
