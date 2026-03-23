---
id: T02
parent: S04
milestone: M007
provides:
  - draw_mcp_panel free function — server list with selection highlight, add-form popup, error line, hint bar
  - handle_mcp_panel_event — full keyboard dispatch for McpPanel screen (a/d/w/Esc/Up/Down/Enter/Tab/Backspace)
  - Add-server form with Tab field switching, Enter confirm, Esc cancel
  - Name uniqueness validation with inline error on duplicate
  - Servers sorted alphabetically after add
  - w key saves to disk and returns to Dashboard; Esc cancels
key_files:
  - crates/assay-tui/src/mcp_panel.rs
  - crates/assay-tui/src/app.rs
key_decisions:
  - "handle_mcp_panel_event extracted as separate App method to avoid borrow-splitting in main handle_event match (consistent with D098)"
  - "w key on save success transitions to Dashboard (not stay on panel) — write-and-close UX consistent with Settings screen (D101)"
  - "Add-form command field captures full string including spaces; args parsing deferred to a future iteration"
patterns_established:
  - "AddServerForm with active_field index for Tab-switching between form fields — reusable for future inline forms"
duration: 0min (already implemented during T01, discovered complete during slice verification)
verification_result: passed
completed_at: 2026-03-23T13:00:00Z
blocker_discovered: false
---

# T02: Draw function, event handling, and wire all keys — make tests green

**MCP panel draw function and full keyboard event handling already implemented during T01; all 4 integration tests pass**

## What Happened

During T01 implementation, the draw function (`draw_mcp_panel`) and all event handling (`handle_mcp_panel_event`) were implemented alongside the types and I/O functions. The auto-mode blocker on T02 was a false alarm — the code was already complete. Verification confirmed all 4 integration tests pass, `cargo test --workspace` is clean, and `just ready` is green.

## Verification

- `cargo test -p assay-tui --test mcp_panel` — 4/4 pass ✓
- `cargo test --workspace` — all pass (exit 0) ✓
- `just ready` — fmt, lint, test, deny all pass ✓
- `cargo build -p assay-tui` — compiles clean ✓

## Deviations

T02 was a no-op — all work was completed during T01. The task split in the plan assumed T01 would only create types/IO/tests and leave rendering/event handling for T02, but the implementation was done as a single pass.

## Files Created/Modified

None — all changes were made during T01.
