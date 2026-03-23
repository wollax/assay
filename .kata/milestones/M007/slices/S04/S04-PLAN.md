# S04: MCP Server Configuration Panel

**Goal:** A TUI panel for managing `.assay/mcp.json` — add, delete, and persist MCP server entries without editing JSON by hand.
**Demo:** Pressing `m` from the dashboard opens `Screen::McpPanel` showing servers from `.assay/mcp.json` (or "none configured" when absent); pressing `a` opens an add-server form (name + command); pressing `d` deletes the selected server; `w` writes changes atomically to `.assay/mcp.json`; `Esc` returns to dashboard.

## Must-Haves

- `McpServerEntry { name, command, args }` struct with Serialize/Deserialize/Clone/Debug derives
- `mcp_config_load(root)` returns `Vec<McpServerEntry>` — empty vec when file missing, parsed servers when valid JSON
- `mcp_config_save(root, servers)` writes `.assay/mcp.json` atomically via NamedTempFile pattern (D093)
- JSON schema: `{ "mcpServers": { "<name>": { "command": "...", "args": [...] } } }` — round-trips cleanly
- `Screen::McpPanel` variant with `servers`, `selected`, `add_form`, `error` fields
- `draw_mcp_panel` free function accepting individual fields + `area: Rect` (D097, D105)
- `m` key from Dashboard opens McpPanel; `a` adds; `d` deletes; `w` writes; `Esc` returns
- Add-server form validates name uniqueness (inline error on duplicate)
- Servers sorted alphabetically by name for deterministic display
- 4 integration tests pass: load-empty, load-from-file, add-writes, delete-writes

## Proof Level

- This slice proves: integration (real filesystem I/O with tempdir fixtures, real App event dispatch)
- Real runtime required: no (tests use tempdir fixtures, not a live TUI terminal)
- Human/UAT required: yes — manual verification of panel rendering and keyboard UX in a real terminal

## Verification

- `cargo test -p assay-tui --test mcp_panel` — all 4 tests pass
- `cargo test --workspace` — no regressions
- `just ready` — fmt, lint, test, deny all pass

## Observability / Diagnostics

- Runtime signals: `error: Option<String>` field on Screen::McpPanel surfaces load/save/validation failures inline
- Inspection surfaces: `.assay/mcp.json` is a human-readable JSON file; `mcp_config_load` can be called from tests or future MCP tools
- Failure visibility: Load errors (invalid JSON, I/O) surface as `error` field on screen; save errors surface identically; duplicate-name validation shows inline error
- Redaction constraints: none — MCP config contains command paths and server names, no secrets

## Integration Closure

- Upstream surfaces consumed: `App`/`Screen` architecture (D089), `draw_*` pattern (D097/D105), atomic NamedTempFile write pattern (D093), `setup_project`/`key` test helpers from `tests/settings.rs`
- New wiring introduced in this slice: `m` key → `Screen::McpPanel` transition; `mcp_panel.rs` module; `serde`/`serde_json` deps on `assay-tui`
- What remains before the milestone is truly usable end-to-end: nothing — S04 is the last slice in M007; after this the milestone definition of done can be verified

## Tasks

- [x] **T01: MCP panel types, JSON I/O, Screen variant, and integration tests** `est:30m`
  - Why: Establishes the data model, file I/O contract, Screen variant, and 4 integration tests that define the acceptance criteria. Tests will initially fail (Screen exists but no event handling yet).
  - Files: `crates/assay-tui/Cargo.toml`, `crates/assay-tui/src/mcp_panel.rs`, `crates/assay-tui/src/lib.rs`, `crates/assay-tui/src/app.rs`, `crates/assay-tui/tests/mcp_panel.rs`
  - Do: Add `serde.workspace = true` and `serde_json.workspace = true` to assay-tui deps. Create `mcp_panel.rs` with `McpServerEntry`, `AddServerForm`, `mcp_config_load`, `mcp_config_save`. Add `Screen::McpPanel` variant to `app.rs`. Add `pub mod mcp_panel` to `lib.rs`. Write 4 integration tests: `mcp_panel_loads_empty_when_no_file`, `mcp_panel_loads_from_mcp_json`, `mcp_panel_add_server_writes_file`, `mcp_panel_delete_server_writes_file`. Verify load/save unit behavior compiles and passes; full test suite may have failures in tests that depend on event handling (T02).
  - Verify: `cargo test -p assay-tui --test mcp_panel` compiles (some tests may fail pending T02 event wiring); `cargo build -p assay-tui` succeeds; `cargo clippy -p assay-tui` clean
  - Done when: `mcp_config_load` and `mcp_config_save` round-trip JSON correctly; `Screen::McpPanel` variant exists; test file compiles with all 4 test functions

- [x] **T02: Draw function, event handling, and wire all keys — make tests green** `est:45m`
  - Why: Implements the actual UI rendering and keyboard interaction that makes the panel usable and passes all 4 integration tests.
  - Files: `crates/assay-tui/src/mcp_panel.rs`, `crates/assay-tui/src/app.rs`
  - Do: Write `draw_mcp_panel(frame, area, servers, selected, add_form, error)` free function — server list with selection highlight, add-form overlay (name + command fields with Tab switching), status/hint line. Wire `m` key in Dashboard arm to load config and transition to `Screen::McpPanel`. Handle keys in McpPanel arm: `a` opens add form, `d` deletes selected, `w` saves via `mcp_config_save`, `Esc` returns to Dashboard (or cancels add form), `Up`/`Down` navigate list, `Enter` confirms add form, `Tab` switches add-form fields. Sort servers alphabetically on load and after add. Validate name uniqueness on add — set `error` field on duplicate.
  - Verify: `cargo test -p assay-tui --test mcp_panel` — all 4 tests pass; `cargo test --workspace` — no regressions; `just ready` — all checks pass
  - Done when: All 4 integration tests pass; `just ready` green; `m` key opens panel, `a`/`d`/`w`/`Esc` work as specified

## Files Likely Touched

- `crates/assay-tui/Cargo.toml`
- `crates/assay-tui/src/lib.rs`
- `crates/assay-tui/src/mcp_panel.rs` (new)
- `crates/assay-tui/src/app.rs`
- `crates/assay-tui/tests/mcp_panel.rs` (new)
