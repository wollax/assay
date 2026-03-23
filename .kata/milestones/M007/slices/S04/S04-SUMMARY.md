---
id: S04
parent: M007
milestone: M007
provides:
  - McpServerEntry struct with Serialize/Deserialize/Clone/Debug
  - AddServerForm struct for inline add-server UI state
  - mcp_config_load — reads .assay/mcp.json, returns sorted entries or empty vec
  - mcp_config_save — atomic JSON write via NamedTempFile (D093 pattern)
  - Screen::McpPanel variant with servers, selected, add_form, error fields
  - draw_mcp_panel free function — server list, add-form popup, error line, hint bar
  - handle_mcp_panel_event — full keyboard dispatch (a/d/w/Esc/Up/Down/Enter/Tab/Backspace)
  - Name uniqueness validation with inline error on duplicate
requires:
  - slice: M006/S01
    provides: App/Screen architecture (D089), draw_* pattern (D097/D105)
  - slice: M006/S04
    provides: Atomic NamedTempFile write pattern (D093)
affects: []
key_files:
  - crates/assay-tui/src/mcp_panel.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/tests/mcp_panel.rs
key_decisions:
  - "D110: MCP panel reads .assay/mcp.json as static config; no live async MCP client"
  - "McpConfigFile uses HashMap<String, McpServerValue> for serde, matching the { mcpServers: { name: { command, args } } } JSON shape"
  - "mcp_config_load returns empty vec (not error) on missing/invalid file — graceful degradation"
  - "handle_mcp_panel_event extracted as separate App method for borrow-splitting (D098)"
  - "w key saves and returns to Dashboard (D101 write-and-close convention)"
patterns_established:
  - "mcp_panel module follows atomic-write pattern (NamedTempFile → sync → persist)"
  - "AddServerForm with active_field index for Tab-switching between form fields"
  - "Integration tests use setup_project + key helpers matching settings.rs pattern"
observability_surfaces:
  - "Screen::McpPanel.error field surfaces I/O and validation errors inline"
  - "mcp_config_save returns Err(String) with descriptive messages on failure"
  - ".assay/mcp.json is a human-readable JSON file inspectable by any tool"
drill_down_paths:
  - .kata/milestones/M007/slices/S04/tasks/T01-SUMMARY.md
  - .kata/milestones/M007/slices/S04/tasks/T02-SUMMARY.md
duration: 15min
verification_result: passed
completed_at: 2026-03-23T13:00:00Z
---

# S04: MCP Server Configuration Panel

**TUI panel for managing .assay/mcp.json — add, delete, persist MCP server entries with atomic writes and inline validation**

## What Happened

Created `mcp_panel.rs` with the full data model (`McpServerEntry`, `AddServerForm`, `McpConfigFile`, `McpServerValue`), atomic JSON I/O (`mcp_config_load`, `mcp_config_save`), and a complete draw function (`draw_mcp_panel`) with server list, selection highlight, add-form popup overlay, error line, and hint bar.

Wired `Screen::McpPanel` variant into the `App` state machine with `handle_mcp_panel_event` handling all keyboard interactions: `m` key from Dashboard opens the panel loading from `.assay/mcp.json`; `a` opens an inline add-server form with Tab field switching; `d` deletes the selected server; `w` saves atomically to disk and returns to Dashboard; `Esc` returns without saving; `Up`/`Down` navigate the server list. Name uniqueness validation shows inline errors on duplicate names.

All implementation was completed in a single pass (T01). T02 was verified as already complete — the auto-mode blocker was a false alarm.

## Verification

- `cargo test -p assay-tui --test mcp_panel` — all 4 tests pass (load-empty, load-from-file, add-writes, delete-writes)
- `cargo test --workspace` — all tests pass (exit 0)
- `just ready` — fmt, lint, test, deny all pass
- `cargo build -p assay-tui` — compiles clean

## Requirements Advanced

- R055 (TUI MCP server management) — MCP panel reads/writes `.assay/mcp.json`; add/delete/save servers from TUI without editing JSON by hand

## Requirements Validated

- R055 — TUI MCP server management proven by 4 integration tests exercising load-empty, load-from-file, add-server-writes-file, delete-server-writes-file; atomic write pattern (D093); inline validation; keyboard UX complete

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

T02 was a no-op — all work (draw function, event handling, keyboard wiring) was completed during T01. The task split assumed T01 would only create types/IO/tests, but the implementation was done as a single pass.

## Known Limitations

- MCP panel is a static config manager — no live MCP server connection or tool inspection (D110, deferred to M008+)
- Add-server form captures command as a single string; `args` field is always empty vec from the form (args can be edited in the JSON file directly)
- No edit-existing-server capability — delete and re-add is the workflow

## Follow-ups

- none

## Files Created/Modified

- `crates/assay-tui/Cargo.toml` — added serde + serde_json workspace deps
- `crates/assay-tui/src/mcp_panel.rs` — new module: types, I/O, draw function
- `crates/assay-tui/src/lib.rs` — added `pub mod mcp_panel`
- `crates/assay-tui/src/app.rs` — Screen::McpPanel variant, m key handler, handle_mcp_panel_event, draw dispatch
- `crates/assay-tui/tests/mcp_panel.rs` — 4 integration tests

## Forward Intelligence

### What the next slice should know
- S04 is the last slice in M007 — there is no next slice in this milestone
- The MCP panel is fully independent of S01/S02/S03 as designed in the boundary map

### What's fragile
- Add-server form only captures name + command; args are always empty — if users need complex server configs with args, they must edit JSON directly

### Authoritative diagnostics
- `cargo test -p assay-tui --test mcp_panel` — 4 tests cover all CRUD operations on mcp.json
- `.assay/mcp.json` on disk is the authoritative config state — always human-readable

### What assumptions changed
- Plan assumed T01 and T02 would be separate work phases — in practice, T01 delivered everything in one pass
