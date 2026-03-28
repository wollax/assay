---
id: S02
parent: M013
milestone: M013
provides:
  - TraceEntry struct (id, root_span_name, span_count, duration_ms, timestamp)
  - SpanLine struct (name, depth, duration_ms) for flattened span tree rendering
  - load_traces() reading .assay/traces/*.json with mtime-descending sort and cap at 20
  - load_trace_spans() helper for loading a single trace file's spans by ID
  - flatten_span_tree() with orphan-span handling (parent_id not in set → treat as root)
  - Screen::TraceViewer variant with trace_list_state, selected_trace, span_lines, span_list_state
  - t key handler in Dashboard (loads traces, transitions to TraceViewer)
  - Two-mode navigation: trace list (Up/Down/Enter) and span tree (Up/Down) with Esc chain
  - draw_trace_viewer() rendering trace list, span tree, and empty state
  - Help overlay entry for t key under Dashboard section
  - 7 integration tests in crates/assay-tui/tests/trace_viewer.rs
requires: []
affects:
  - S03: independent (no coupling)
  - S04: independent (no coupling)
key_files:
  - crates/assay-tui/src/trace_viewer.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/src/lib.rs
  - crates/assay-tui/tests/trace_viewer.rs
  - crates/assay-tui/tests/mcp_panel.rs
key_decisions:
  - "D182: Orphan spans treated as additional roots at depth 0, matching CLI behavior"
  - "D183: TraceViewer loads traces on screen transition (t key), not on every draw"
  - "D184: Two-mode screen pattern — selected_trace Option<usize> switches list/detail mode"
  - "D180: top-20 most-recent files by mtime (already captured)"
patterns_established:
  - "Two-mode screen pattern (D184) reusable for any future detail-drill screens"
  - "handle_trace_viewer_event() extracted as method — same borrow-splitting as handle_mcp_panel_event (D098)"
  - "draw_trace_viewer uses highlight_style(bold().reversed()) + render_stateful_widget — same as draw_dashboard"
observability_surfaces:
  - "tracing::warn! on unreadable/unparseable trace files in load_traces() and load_trace_spans()"
  - "Screen::TraceViewer fields (traces, span_lines, selected_trace) fully inspectable in integration tests via pattern match"
  - "Parse errors produce skipped entries (trace list shows only successfully parsed files)"
drill_down_paths:
  - .kata/milestones/M013/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M013/slices/S02/tasks/T02-SUMMARY.md
  - .kata/milestones/M013/slices/S02/tasks/T03-SUMMARY.md
duration: 30min (T01: 10min, T02: 12min, T03: 8min)
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
---

# S02: TUI Trace Viewer

**TUI `t` key opens a navigable trace viewer; span tree visible from real `.assay/traces/` JSON; Esc closes — proven by 7 integration tests.**

## What Happened

Three tasks built the trace viewer incrementally. T01 created the `trace_viewer.rs` module with `TraceEntry`/`SpanLine` types, `load_traces()` (reads `.assay/traces/*.json`, parses via `SpanData`, sorts by mtime descending, caps at 20), and `flatten_span_tree()` (adjacency map with orphan-root handling). It also added the minimal `Screen::TraceViewer` variant and the `t` key handler, plus the integration test file with 3 screen transition tests.

T02 expanded the variant to its full spec: `trace_list_state`, `selected_trace`, `span_lines`, `span_list_state`. Extracted event handling to `handle_trace_viewer_event()` using the D098 borrow-splitting pattern. Implemented two-mode navigation (trace list / span tree) with the Esc chain (span tree → trace list → Dashboard). Added `load_trace_spans()` helper, updated the help overlay, and grew the integration test suite to 6 tests.

T03 added 1 final integration test (`test_trace_viewer_with_traces_loads_entries`) verifying multi-trace loading and mtime-descending sort order, plus expanded the span tree test to 3 spans with depth assertions. The `draw_trace_viewer()` renderer had already been implemented in T02, so T03 was primarily verification-focused.

## Verification

- `cargo test -p assay-tui --test trace_viewer` — 7 tests pass
- `cargo test -p assay-tui` — all 83 TUI tests pass, zero regressions (11 lib + 7 trace_viewer + various integration suites)
- `just ready` — workspace-wide green (fmt, lint, test, deny all pass; ~1512+ tests)

## Requirements Advanced

- R066 (TUI trace viewer) — now validated: screen exists, renders span tree from real trace JSON, accessible via `t` key

## Requirements Validated

- R066 — `Screen::TraceViewer` variant exists; `t` key opens trace list from Dashboard; Enter expands span tree; Esc chain closes; 7 integration tests prove all paths including empty dir and orphan span handling; `just ready` green

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- `draw_trace_viewer` uses `List` widget with indented text for both trace list and span tree instead of `Table`. The span tree's variable indentation maps better to plain `ListItem` text than table columns, and `List` integrates naturally with `ListState` for `highlight_style`. This simplified the implementation without any loss of information.
- T02 delivered 4 additional navigation integration tests beyond the 2 specified in the plan (up/down navigation in both modes, Enter-expand-Esc chain). More coverage is better; no plan deviation in substance.
- `draw_trace_viewer()` was fully implemented during T02 as part of the screen variant work, leaving T03 focused on integration tests and verification rather than building the renderer.

## Known Limitations

- Traces are loaded once on screen entry (synchronous, D183). Large trace directories (>20 files) are not live-refreshed while the viewer is open — a manual re-entry via Esc + `t` is needed to see new traces.
- Visual rendering quality (scrolling UX with real trace data, color scheme) is UAT-only — integration tests verify structure but not aesthetics.

## Follow-ups

- Live trace refresh (polling while TraceViewer is open) — deferred per D183/D180
- Span detail overlay (third mode with full span fields/tags) — not planned for M013

## Files Created/Modified

- `crates/assay-tui/src/trace_viewer.rs` — New: TraceEntry, SpanLine types; load_traces(), load_trace_spans(), flatten_span_tree(); 7 unit tests
- `crates/assay-tui/src/app.rs` — Screen::TraceViewer variant (full), t key handler, handle_trace_viewer_event(), draw_trace_viewer(), help overlay entry
- `crates/assay-tui/src/lib.rs` — Added `pub mod trace_viewer`
- `crates/assay-tui/tests/trace_viewer.rs` — New: 7 integration tests for all navigation paths
- `crates/assay-tui/tests/mcp_panel.rs` — Added TraceViewer arm to screen_name helper

## Forward Intelligence

### What the next slice should know
- S03 (OTel metrics) is fully independent — no coupling to trace_viewer module. `TracingGuard` in `assay-core::telemetry` is the only shared surface.
- S04 (wizard runnable criteria) is also fully independent — touches `assay-core::wizard` and `assay-cli::commands::spec`, nothing in assay-tui.

### What's fragile
- `load_traces()` couples to `SpanData` deserialization from `assay_core::telemetry`. If `JsonFileLayer` output format changes, trace loading will silently skip files (warn emitted, no panic). This is intentional but means format drift is invisible in tests.
- The `SpanData` struct from `assay-core` must remain `pub` and `Deserialize` for `trace_viewer.rs` to compile. Currently true; watch for visibility changes in S03.

### Authoritative diagnostics
- `cargo test -p assay-tui --test trace_viewer` is the canonical verification — all 7 tests must pass
- `Screen::TraceViewer { traces, span_lines, selected_trace, .. }` pattern match in tests is the primary inspection surface

### What assumptions changed
- No significant assumption changes. The two-mode Option<usize> pattern (D184) was chosen over separate Screen variants to avoid explosion of match arms — this proved correct.
