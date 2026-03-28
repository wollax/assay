---
id: T01
parent: S02
milestone: M013
provides:
  - TraceEntry struct for trace list display
  - SpanLine struct for flattened span tree rendering
  - load_traces() function reading .assay/traces/*.json with mtime sort and cap at 20
  - flatten_span_tree() with orphan span handling
  - Screen::TraceViewer variant with t-key navigation and Esc back to Dashboard
key_files:
  - crates/assay-tui/src/trace_viewer.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/tests/trace_viewer.rs
key_decisions:
  - "Orphan spans (parent_id referencing non-existent span_id) treated as roots at depth 0, matching CLI behavior"
  - "TraceViewer screen stores loaded traces in enum variant (load on transition, not on every draw)"
patterns_established:
  - "trace_viewer module follows same adjacency-map pattern as CLI traces.rs for span tree flattening"
  - "Screen transition for t key follows exact same pattern as a key (Analytics) — guard on project_root, load data, set Screen variant"
observability_surfaces:
  - "tracing::warn! on unreadable/unparseable trace files in load_traces()"
duration: 10min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T01: Integration tests and trace_viewer module scaffold

**Added trace_viewer module with TraceEntry/SpanLine types, load_traces/flatten_span_tree logic, Screen::TraceViewer variant, and integration tests for screen transitions**

## What Happened

Created `crates/assay-tui/src/trace_viewer.rs` with core data types (`TraceEntry`, `SpanLine`) and two main functions: `load_traces()` reads `.assay/traces/*.json`, parses each via `SpanData`, sorts by filesystem mtime descending, and caps at 20 entries; `flatten_span_tree()` builds an adjacency map and recursively flattens with depth tracking, treating orphan spans (parent_id referencing a non-existent span_id) as additional roots.

Added `Screen::TraceViewer { traces, selected }` variant to the `Screen` enum. Wired the `t` key in Dashboard to load traces and transition to the new screen, with Esc returning to Dashboard. Added a minimal `draw_trace_viewer` renderer showing a table of traces.

Created integration test file `crates/assay-tui/tests/trace_viewer.rs` with three screen transition tests that all pass. Updated `tests/mcp_panel.rs` screen_name helper to include the new variant.

## Verification

- `cargo test -p assay-tui --lib` — all 11 unit tests pass (7 trace_viewer + 4 mcp_panel)
- `cargo test -p assay-tui --test trace_viewer` — all 3 integration tests pass
- `cargo test -p assay-tui` — all 80 TUI tests pass, zero regressions
- `just ready` — 1512 tests pass workspace-wide, fmt/lint/deny all green

### Slice-level verification (partial):
- ✅ `cargo test -p assay-tui --test trace_viewer` — integration tests pass
- ✅ `cargo test -p assay-tui` — all existing TUI tests still pass
- ✅ `just ready` — workspace-wide green

## Diagnostics

- `load_traces()` emits `tracing::warn!` when a trace file is unreadable or unparseable, with path and error details
- `flatten_span_tree()` output is a plain `Vec<SpanLine>` — easily inspectable in tests
- Parse errors result in skipped entries (no panic, no empty screen)

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/src/trace_viewer.rs` — New module with TraceEntry, SpanLine, load_traces(), flatten_span_tree(), and unit tests
- `crates/assay-tui/src/lib.rs` — Added `pub mod trace_viewer`
- `crates/assay-tui/src/app.rs` — Added Screen::TraceViewer variant, t-key handler, draw_trace_viewer, event handling
- `crates/assay-tui/tests/trace_viewer.rs` — Integration tests for screen transitions
- `crates/assay-tui/tests/mcp_panel.rs` — Added TraceViewer arm to screen_name helper
