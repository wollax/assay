---
id: T02
parent: S02
milestone: M013
provides:
  - Full Screen::TraceViewer variant with trace_list_state, selected_trace, span_lines, span_list_state fields
  - t key handler from Dashboard loading traces and transitioning to TraceViewer
  - Two-mode navigation: trace list (Up/Down/Enter) and span tree (Up/Down) with Esc chain (span→list→Dashboard)
  - load_trace_spans() helper for loading a single trace file's spans by ID
  - Help overlay entry for t key under Dashboard section
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/src/trace_viewer.rs
  - crates/assay-tui/tests/trace_viewer.rs
key_decisions:
  - "TraceViewer event handling extracted to handle_trace_viewer_event() method (same borrow-splitting pattern as handle_mcp_panel_event, D098)"
  - "Trace list uses ListState with highlight_style for selection instead of manual index-based row styling"
  - "Span tree rendering uses List widget with indented text instead of Table — simpler for variable-depth tree"
patterns_established:
  - "Two-mode screen pattern: selected_trace Option<usize> switches between list and detail view within same Screen variant"
  - "draw_trace_viewer uses &mut ListState re-borrow from &mut self.screen inside match (same as dashboard pattern)"
observability_surfaces:
  - "tracing::warn! on failed trace file reads in load_trace_spans()"
  - "Screen::TraceViewer fields inspectable in integration tests via pattern match on app.screen"
duration: 12min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T02: Screen::TraceViewer variant and event handling

**Expanded TraceViewer to full two-mode screen with ListState navigation, Enter-to-expand span tree, Esc chain back to Dashboard, and load_trace_spans() helper**

## What Happened

Replaced the minimal TraceViewer variant (traces + selected usize) with the full spec: trace_list_state, selected_trace, span_lines, span_list_state. The `t` key handler initializes ListState with select(Some(0)) for non-empty traces. Event handling was extracted to `handle_trace_viewer_event()` using the same borrow-splitting pattern as MCP panel (D098), supporting trace list mode (Up/Down/Enter/Esc/q/slash) and span tree mode (Up/Down/Esc/q). The `draw_trace_viewer` function now renders either a List of trace entries or an indented span tree depending on `selected_trace`. Added `load_trace_spans()` to trace_viewer.rs for loading a single trace file. Updated help overlay with `t` key entry and incremented height.

## Verification

- `cargo test -p assay-tui --test trace_viewer` — 6 tests pass (test_t_key_transitions_to_trace_viewer, test_esc_from_trace_list_returns_to_dashboard, test_empty_traces_dir_shows_trace_viewer, test_enter_expands_span_tree_and_esc_returns, test_up_down_navigation_in_trace_list, test_up_down_navigation_in_span_tree)
- `cargo test -p assay-tui` — all 82 tests pass, zero regressions

## Diagnostics

- Pattern-match on `app.screen` as `Screen::TraceViewer { trace_list_state, selected_trace, span_lines, span_list_state, .. }` in tests to inspect all navigation state
- load_trace_spans() emits tracing::warn! with path and error on read/parse failures

## Deviations

- Changed draw_trace_viewer from Table-based rendering to List-based rendering for both trace list and span tree — List widget integrates naturally with ListState for highlight_style, and the span tree's variable indentation maps better to plain ListItem text than table columns
- Added 4 new integration tests beyond the 2 specified in must-haves (navigation tests for both modes, enter-expand-esc chain test)

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — Expanded Screen::TraceViewer variant, added handle_trace_viewer_event(), updated draw dispatch and draw_trace_viewer(), added t to help overlay
- `crates/assay-tui/src/trace_viewer.rs` — Added load_trace_spans() helper
- `crates/assay-tui/tests/trace_viewer.rs` — Updated existing tests for new variant structure, added 4 new tests for navigation and span tree expansion
