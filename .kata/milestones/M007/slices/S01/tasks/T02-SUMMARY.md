---
id: T02
parent: S01
milestone: M007
provides:
  - handle_agent_line real implementation (cap at 10 000, no-op on non-AgentRun)
  - handle_agent_done real implementation (Done/Failed transition + disk refresh)
  - draw_agent_run free function (scrollable line list, status bar, "Starting…" placeholder)
  - Screen::AgentRun handle_event arm (Esc → Dashboard, j/↓ scroll down, k/↑ scroll up)
key_files:
  - crates/assay-tui/src/app.rs
key_decisions:
  - draw_agent_run is a free function (not a method) consistent with D097/D105 pattern — individual fields passed explicitly
  - Block::inner() used to get inner area before rendering the block widget; split inner into content (Fill) + status (1 row)
  - scroll_offset is mutated directly in the AgentRun arm (no clone-then-mutate needed — scroll is self-contained)
patterns_established:
  - cap-at-10k via push + remove(0): simple Vec is sufficient per research doc guidance
  - handle_agent_done refreshes milestones + cycle_slug using .ok() graceful degradation, matching with_project_root pattern
observability_surfaces:
  - Screen::AgentRun { lines, status } — lines accumulates all subprocess stdout (capped at 10 000); status holds final exit code in Done/Failed variants
  - draw_agent_run renders "● Running…", "✓ Done (exit N)", or "✗ Failed (exit N)" in bottom status bar
duration: ~20 min
verification_result: passed
completed_at: 2026-03-21
blocker_discovered: false
---

# T02: Implement `handle_agent_line`, `handle_agent_done`, and `draw_agent_run`

**Replaced all T01 stubs with real implementations; all 8 agent_run integration tests and all 27 pre-existing TUI tests pass.**

## What Happened

Implemented the three key methods and updated the draw/event dispatch arms in `app.rs`:

1. **`handle_agent_line`** — `if let Screen::AgentRun` pattern-match; push line; trim front if `> 10_000`. No-op on all other screen variants.

2. **`handle_agent_done`** — sets `*status = Done { exit_code }` or `Failed { exit_code }` based on exit code zero/non-zero; then unconditionally attempts `milestone_scan` + `cycle_status` refresh (guarded on `project_root.is_some()`), degrading gracefully with `.ok()` on I/O error.

3. **`draw_agent_run`** (free fn, replaces `draw_agent_run_stub`) — renders a bordered Block with title `" Agent Run: {slug} "`; uses `Block::inner()` to split into content area (scrollable line list) and a 1-row status bar. Empty lines → "Starting…" placeholder (dim). Status bar: yellow for Running, green for Done, red for Failed, each with `Esc: back` hint.

4. **`Screen::AgentRun` arm in `handle_event()`** — `Esc → Screen::Dashboard`, `Down/j → scroll_offset.saturating_add(1)`, `Up/k → scroll_offset.saturating_sub(1)`. Direct field mutation worked cleanly without clone-then-mutate.

5. **`draw()` arm** — destructures fields from `&self.screen` and calls `draw_agent_run` directly; no cloning needed.

## Verification

```
cargo test -p assay-tui --test agent_run
# 8/8 pass: handle_agent_line_accumulates, handle_agent_done_zero, handle_agent_done_nonzero,
#            handle_agent_line_caps_at_ten_thousand, handle_agent_line_noops,
#            r_key_noops_when_event_tx_is_none, launch_agent_streaming_delivers_all_lines,
#            launch_agent_streaming_delivers_exit_code

cargo test -p assay-tui
# 35 total: 8 agent_run + 27 pre-existing (settings, spec_browser, wizard_round_trip, etc.)
# All pass.

cargo check -p assay-tui
# Clean compile, no warnings.
```

## Diagnostics

- `app.screen` discriminant `Screen::AgentRun { ref lines, ref status, .. }` — lines holds full stdout buffer; status holds `Done { exit_code }` or `Failed { exit_code }` after agent exits.
- TUI renders bottom bar: "● Running…", "✓ Done (exit 0)", or "✗ Failed (exit N)" — visible without code inspection.

## Deviations

None. All steps followed the plan exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — replaced stubs with real `handle_agent_line`, `handle_agent_done`, `draw_agent_run`; updated `draw()` and `handle_event()` AgentRun arms
