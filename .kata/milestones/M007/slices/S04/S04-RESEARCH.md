# S04: MCP Server Configuration Panel — Research

**Date:** 2026-03-23
**Domain:** Ratatui TUI panel, JSON file I/O
**Confidence:** HIGH

## Summary

S04 is a self-contained UI panel for managing `.assay/mcp.json` — a static config file listing MCP servers (name + command + args). Per D110, there is no live async MCP client; the panel is a JSON config editor with list navigation, add/delete operations, and atomic file writes. The scope is narrow and well-bounded by existing patterns.

The key implementation challenge is that `serde_json` is not yet a dependency of `assay-tui`. It must be added (workspace dep exists). The MCP JSON schema is minimal: `{ "mcpServers": { "<name>": { "command": "...", "args": [...] } } }`. All I/O patterns (atomic NamedTempFile write, screen dispatch, draw function, integration tests with tempdir fixtures) are thoroughly established by S01–S03 and M006.

The panel follows the exact same architecture as `Screen::Settings`: full-screen view, list navigation with Up/Down, action keys (`a` add, `d` delete, `w` write), Esc to return. An inline add-form (name + command fields) is the only new UI pattern — but it mirrors the wizard's multi-step text input approach from M006/S02.

## Recommendation

Follow the Settings screen pattern exactly:
1. New module `crates/assay-tui/src/mcp_panel.rs` — types (`McpServerEntry`, `AddServerForm`, `McpPanelState`), load/save functions, draw function, event handler
2. Add `Screen::McpPanel` variant to the `Screen` enum
3. Wire `m` key in Dashboard to open `Screen::McpPanel`
4. Add `serde_json` + `serde` dependencies to `assay-tui/Cargo.toml`
5. 4 integration tests in `tests/mcp_panel.rs`

Keep all MCP panel logic in the new module — don't pollute `app.rs` with MCP-specific types. `app.rs` only needs the Screen variant and a small event/draw dispatch block.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Atomic file writes | `NamedTempFile::new_in + write_all + sync_all + persist` pattern (D093, `config::save`) | Battle-tested, prevents partial writes; same pattern used across 6+ files in codebase |
| JSON serialization | `serde_json` workspace dep (already used by assay-core, assay-types, assay-harness) | Standard; no reason to hand-parse JSON |
| Integration test fixtures | `tempfile::TempDir` + `setup_project()` pattern (from `tests/settings.rs`) | Consistent test infrastructure across all TUI test files |

## Existing Code and Patterns

- `crates/assay-tui/src/app.rs` (Screen::Settings) — Full-screen config panel pattern: `selected` index for list nav, `error: Option<String>` for inline errors, `w` saves, `Esc` cancels. **Follow this exactly.**
- `crates/assay-core/src/config/mod.rs` (`save()` fn at line 227) — Atomic NamedTempFile write pattern for config files. The MCP panel's `mcp_config_save` should replicate this pattern.
- `crates/assay-tui/src/slash.rs` — Standalone module with types, logic, draw fn, event handler all in one file. Good template for module structure.
- `crates/assay-tui/tests/settings.rs` — Test pattern: `setup_project()` helper, `key()` helper, `App::with_project_root`, assert on `app.screen` variants. MCP tests should follow identically.
- `crates/assay-tui/src/wizard.rs` — Multi-field text input with cursor tracking (`WizardState`). The add-server form needs similar char input + backspace handling, but simpler (2 fields, no multi-step).

## Constraints

- **No serde/serde_json in assay-tui yet** — Must add `serde.workspace = true` and `serde_json.workspace = true` to `crates/assay-tui/Cargo.toml`. Both are workspace deps.
- **D001: zero traits** — All functions must be free functions. No `Widget` trait impls, no custom traits.
- **D093: Atomic writes** — `mcp_config_save` must use NamedTempFile pattern, not direct `fs::write`.
- **D097: Draw fns take individual fields, not `&mut App`** — `draw_mcp_panel(frame, area, ...)` must accept explicit params.
- **D105: All draw_* accept `area: Rect`** — The MCP panel draw fn gets `content_area` from `App::draw()`.
- **D110: No live MCP client** — Panel reads/writes `.assay/mcp.json` only. No async, no tokio, no connection attempts.
- **D089: App struct with Screen enum** — McpPanel state should be in the Screen variant (like Settings), not in separate App-level fields (the detail_* pattern is for screens that share data across navigation).
- **`.assay/` directory must exist** — The panel only activates when `project_root` is Some (Dashboard is only reachable when project exists). Save should create `.assay/mcp.json` if absent, but `.assay/` dir is guaranteed.
- **JSON schema** — The boundary map specifies: `{ "mcpServers": { "<name>": { "command": "...", "args": [...] } } }`. This is the MCP standard config format used by Claude Code, Kata, etc. Must round-trip cleanly.

## Common Pitfalls

- **Borrow checker with Screen enum variants** — When `Screen::McpPanel { .. }` contains mutable state and `draw()` needs to read it while also borrowing `self` for other things: use `..` pattern in match arms and pass fields explicitly to draw functions (D098). Clone data into locals in `handle_event` before mutating `self.screen`.
- **Missing serde derives on McpServerEntry** — The struct needs `#[derive(Serialize, Deserialize, Clone, Debug)]` for JSON round-trip. Easy to forget `Clone` which is needed when building the Vec for save.
- **Empty file vs missing file** — `mcp_config_load` must handle: (1) file doesn't exist → empty vec, (2) file exists but is empty → empty vec or error, (3) file exists with valid JSON → parse, (4) file exists with invalid JSON → error string. The boundary map says "returns empty vec" for missing file.
- **HashMap ordering in JSON** — `mcpServers` is a JSON object (HashMap). Server ordering in the list may not match file ordering. Accept this — JSON objects are unordered by spec. The list should sort servers alphabetically by name for deterministic display.
- **Name uniqueness** — When adding a server, validate the name doesn't already exist. Display inline error, don't overwrite silently.

## Open Risks

- **Add-form UX complexity** — The add-server form needs two text fields (name, command) with field switching (Tab or Up/Down). This is a new pattern — the wizard uses single-field steps. Risk: getting the cursor/focus switching right. Mitigation: keep it simple — just two fields with Tab to switch, Enter to confirm, Esc to cancel.
- **Args parsing** — The boundary map shows `args: Vec<String>` but the add form collects a single command string. Decision needed: accept a space-separated command+args string and split, or have separate fields. Recommendation: single "command" field that stores the full binary path; args defaults to empty vec. Users can edit `mcp.json` directly for complex args. Keep the form minimal for M007; enhanced editing in M008+.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Ratatui | `blacktop/dotfiles@ratatui-tui` (57 installs) | available — not needed (existing codebase has thorough Ratatui patterns) |
| Ratatui | `padparadscho/skills@rs-ratatui-crate` (22 installs) | available — not needed |

No skills needed — the codebase already has 6+ working Ratatui screen implementations to follow.

## Sources

- Codebase exploration: `app.rs` Screen enum (1507 lines), `slash.rs` module pattern, `config/mod.rs` atomic save, `settings.rs` test pattern
- Boundary map in M007-ROADMAP.md — defines exact types and test names for S04
- D110 (no live MCP client), D093 (atomic writes), D097/D105 (draw fn signatures), D089 (App/Screen architecture)
