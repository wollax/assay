---
id: T03
parent: S02
milestone: M013
provides:
  - draw_trace_viewer() rendering trace list, span tree, and empty state
  - Integration test for multi-trace loading with mtime sorting verification
  - Integration test for Enter→span tree (3 spans)→Esc→trace list navigation
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/tests/trace_viewer.rs
key_decisions: []
patterns_established:
  - "draw_trace_viewer uses same highlight_style(bold().reversed()) + render_stateful_widget pattern as draw_dashboard"
observability_surfaces:
  - "Screen::TraceViewer fields (traces, span_lines, selected_trace) are fully inspectable in integration tests via pattern match"
duration: 8min
verification_result: passed
completed_at: 2026-03-28T00:00:00Z
blocker_discovered: false
---

# T03: Render trace list and span tree

**Added integration tests for trace loading/sorting and span tree navigation; draw_trace_viewer already implemented in T02**

## What Happened

The `draw_trace_viewer()` function and its wiring into `App::draw()` were already implemented during T02 as part of the screen variant work. The rendering handles all three states: empty (informative message), trace list (timestamp + root span + span count + duration per entry with highlight), and span tree (indented span names with duration). This task focused on adding the two specific integration tests required by the plan.

Added `test_trace_viewer_with_traces_loads_entries`: creates 2 trace fixtures, verifies both load, verifies mtime-descending sort order, and checks span_count/duration_ms fields.

Updated `test_enter_expands_span_tree_and_esc_returns`: expanded from 2 to 3 spans (root + 2 children) to match plan spec, added depth assertions for each span line.

## Verification

- `cargo test -p assay-tui --test trace_viewer` — 7 tests pass (6 existing + 1 new)
- `cargo test -p assay-tui` — all TUI tests pass, no regressions
- `just ready` — green (fmt, lint, test, deny all pass)

## Diagnostics

Pattern-match on `Screen::TraceViewer { traces, span_lines, selected_trace, .. }` in tests to inspect all rendering state. Empty state renders an informative message rather than a blank screen.

## Deviations

`draw_trace_viewer()` and its `App::draw()` wiring were already implemented in T02. T03 focused on the integration tests and verification rather than re-implementing the renderer.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/tests/trace_viewer.rs` — Added `test_trace_viewer_with_traces_loads_entries`, updated `test_enter_expands_span_tree_and_esc_returns` with 3-span fixture and depth assertions
