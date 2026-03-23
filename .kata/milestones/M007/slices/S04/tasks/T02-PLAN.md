---
estimated_steps: 5
estimated_files: 2
---

# T02: Draw function, event handling, and wire all keys — make tests green

**Slice:** S04 — MCP Server Configuration Panel
**Milestone:** M007

## Description

Implement `draw_mcp_panel` rendering and wire all keyboard event handling for the MCP panel in `app.rs`. This makes the panel fully functional: `m` opens it from Dashboard, list navigation works, `a` opens add-form, `d` deletes, `w` saves, `Esc` returns. All 4 integration tests must pass and `just ready` must be green.

## Steps

1. Write `draw_mcp_panel(frame, area, servers, selected, add_form, error)` free function in `mcp_panel.rs`:
   - Full-screen bordered block titled " MCP Server Configuration "
   - Server list with highlight on selected row (name + command displayed per row)
   - "No servers configured" message when list is empty
   - Hint line at bottom: `a:add  d:delete  w:save  Esc:back`
   - When `add_form` is `Some`, render an inline form overlay (centered popup) with name + command fields, active field highlighted, Tab switches, Enter confirms, Esc cancels
   - When `error` is `Some`, render error text in red above the hint line
2. Wire `m` key in `App::handle_event` Dashboard arm:
   - Call `mcp_config_load(project_root)` to get servers
   - Transition to `Screen::McpPanel { servers, selected: 0, add_form: None, error: None }`
3. Wire event handling in `App::handle_event` for `Screen::McpPanel { .. }` arm:
   - `Esc`: if `add_form` is Some → set to None (cancel add); else → transition to `Screen::Dashboard`
   - `Up`/`Down`: navigate `selected` index (clamped to server list length)
   - `a`: set `add_form = Some(AddServerForm::new())`
   - `d`: remove server at `selected` index if non-empty; adjust `selected` if needed
   - `w`: call `mcp_config_save`; on success transition to Dashboard; on error set `error` field
   - When `add_form` is Some, intercept `Char`, `Backspace`, `Tab`, `Enter`:
     - `Char(c)`: append to active field buffer
     - `Backspace`: pop from active field buffer
     - `Tab`: toggle `active_field` between 0 and 1
     - `Enter`: validate name uniqueness (set `error` if duplicate), then push new `McpServerEntry` to `servers`, sort alphabetically, clear `add_form`
4. Wire `draw_mcp_panel` call in `App::draw()` match arm for `Screen::McpPanel { .. }`:
   - Use `..` pattern to avoid borrow-split (D098)
   - Pass fields explicitly: `&self.` won't work here since fields are in enum variant — clone/extract into locals before the match, or read directly from the match binding
5. Run `cargo test -p assay-tui --test mcp_panel` — all 4 tests must pass. Run `just ready` — all checks must pass. Fix any clippy warnings or formatting issues.

## Must-Haves

- [ ] `draw_mcp_panel` renders server list, selection highlight, add-form overlay, error line, and hint line
- [ ] `m` key from Dashboard transitions to `Screen::McpPanel` with loaded servers
- [ ] `a` opens add-form; `Enter` confirms with name-uniqueness validation; `Esc` cancels
- [ ] `d` deletes selected server; `Up`/`Down` navigate
- [ ] `w` saves via `mcp_config_save` and returns to Dashboard; error displayed inline on failure
- [ ] All 4 integration tests in `tests/mcp_panel.rs` pass
- [ ] `just ready` green (fmt, lint, test, deny)

## Verification

- `cargo test -p assay-tui --test mcp_panel` — all 4 pass
- `cargo test --workspace` — no regressions
- `just ready` — fmt, lint, test, deny all pass

## Observability Impact

- Signals added/changed: `error` field on McpPanel set on save failure or duplicate-name validation
- How a future agent inspects this: Assert on `app.screen` variant fields after driving key events; read `.assay/mcp.json` after `w` save
- Failure state exposed: Inline error string visible in Screen::McpPanel.error; save errors include I/O detail

## Inputs

- `crates/assay-tui/src/mcp_panel.rs` — types and I/O from T01
- `crates/assay-tui/src/app.rs` — existing handle_event/draw patterns from Settings, SlashCmd, AgentRun screens
- `crates/assay-tui/tests/mcp_panel.rs` — 4 tests from T01 that must pass

## Expected Output

- `crates/assay-tui/src/mcp_panel.rs` — extended with `draw_mcp_panel` function
- `crates/assay-tui/src/app.rs` — `m` key handler in Dashboard, `Screen::McpPanel` event handling, `draw_mcp_panel` call in `draw()`
- All 4 `mcp_panel` tests green; `just ready` green
