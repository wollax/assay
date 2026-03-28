# S02: TUI Trace Viewer

**Goal:** Add a trace viewer screen to the TUI accessible via `t` from Dashboard, showing a list of recent traces and a navigable span tree.
**Demo:** User presses `t` on the Dashboard → sees a list of up to 20 recent traces with timestamp, root span name, span count, and duration. Pressing Enter on a trace shows an indented span tree with timing. Esc returns to trace list, second Esc returns to Dashboard.

## Must-Haves

- `t` key from Dashboard opens `Screen::TraceViewer`
- Trace list shows up to 20 most-recent `.assay/traces/*.json` files sorted by mtime (D180)
- Each trace entry shows: timestamp, root span name, span count, duration
- Enter on a trace expands the span tree (flattened with indentation and timing)
- Up/Down navigates in both trace list and span tree
- Esc from span tree returns to trace list; Esc from trace list returns to Dashboard
- Empty state shows informative message when no traces exist
- Orphan-span guard: spans whose `parent_id` doesn't match any `span_id` are treated as roots
- Help overlay lists `t` key under Dashboard section
- Integration test proves `t`→TraceViewer transition and Esc→Dashboard return

## Proof Level

- This slice proves: integration
- Real runtime required: no (tests use fixture JSON files, not live tracing)
- Human/UAT required: yes (visual rendering quality, scrolling UX with real trace data)

## Verification

- `cargo test -p assay-tui --test trace_viewer` — integration tests for screen transitions, trace loading, span tree navigation, empty state, orphan-span handling
- `cargo test -p assay-tui` — all existing TUI tests still pass (no regressions)
- `just ready` — workspace-wide green

## Observability / Diagnostics

- Runtime signals: `tracing::warn!` on unreadable trace files (skipped gracefully), `tracing::debug!` on trace load count
- Inspection surfaces: `Screen::TraceViewer` variant is inspectable from integration tests via `app.screen` pattern match
- Failure visibility: parse errors surface as skipped entries (trace list shows only successfully parsed files; count of skipped files could be shown if non-zero)
- Redaction constraints: none (trace data contains no secrets)

## Integration Closure

- Upstream surfaces consumed: `assay_core::telemetry::SpanData` (pub struct, Serialize+Deserialize), `.assay/traces/*.json` files written by `JsonFileLayer` (M009/S04)
- New wiring introduced in this slice: `Screen::TraceViewer` variant + `t` key handler + `draw_trace_viewer()` render function + `trace_viewer.rs` module
- What remains before the milestone is truly usable end-to-end: S03 (OTel metrics), S04 (wizard runnable criteria) — independent of this slice

## Tasks

- [x] **T01: Integration tests and trace_viewer module scaffold** `est:45m`
  - Why: Test-first — define the contract before building the UI. Creates the integration test file and the trace_viewer module with types and trace-loading logic.
  - Files: `crates/assay-tui/tests/trace_viewer.rs`, `crates/assay-tui/src/trace_viewer.rs`, `crates/assay-tui/src/lib.rs`
  - Do: Create `trace_viewer.rs` with `TraceEntry` struct, `load_traces()` fn (reads dir, parses JSON, sorts by mtime, caps at 20), `flatten_span_tree()` fn (builds adjacency map, handles orphan roots, returns indented lines). Create integration test with fixture JSON exercising: `t` opens TraceViewer, Esc returns to Dashboard, load_traces returns correct entries, orphan span handling, empty dir handling. Tests will initially fail (Screen variant doesn't exist yet).
  - Verify: `cargo test -p assay-tui --test trace_viewer` compiles; trace_viewer module functions are unit-testable standalone
  - Done when: `load_traces()` and `flatten_span_tree()` pass their unit tests; integration test file exists and compiles (screen transition tests may fail until T02 wires them)

- [ ] **T02: Screen::TraceViewer variant and event handling** `est:45m`
  - Why: Wire the trace viewer into the TUI app — Screen variant, `t` key handler, navigation (Up/Down/Enter/Esc), and list state management.
  - Files: `crates/assay-tui/src/app.rs`, `crates/assay-tui/src/trace_viewer.rs`
  - Do: Add `Screen::TraceViewer` variant to `Screen` enum with fields: `traces: Vec<TraceEntry>`, `trace_list_state: ListState`, `selected_trace: Option<usize>` (index into traces for expanded span tree), `span_lines: Vec<SpanLine>`, `span_list_state: ListState`. Add `t` key handler in Dashboard match arm: load traces via `load_traces()`, transition to TraceViewer. Add TraceViewer match arm in `handle_event()`: Up/Down for list navigation, Enter to expand span tree (calls `flatten_span_tree`), Esc to return (span tree → trace list → Dashboard). Add `'t'` to help overlay Dashboard section.
  - Verify: `cargo test -p assay-tui --test trace_viewer` — screen transition tests pass
  - Done when: `t` opens TraceViewer, Enter expands span tree, Esc chain works (span→list→Dashboard), help overlay updated

- [ ] **T03: Render trace list and span tree** `est:45m`
  - Why: Build the visual rendering — the actual UI the user sees. Makes the integration tests fully pass.
  - Files: `crates/assay-tui/src/trace_viewer.rs`, `crates/assay-tui/src/app.rs`
  - Do: Implement `draw_trace_viewer()` free function (D097/D105 pattern — takes `frame, area, traces, trace_list_state, selected_trace, span_lines, span_list_state`). Trace list view: bordered block titled "Traces", each item shows `timestamp  root_span  spans:N  duration_ms`. Span tree view: bordered block titled with trace root span name, each line indented with `"  ".repeat(depth)` + span name + duration. Empty state: centered paragraph "No traces found. Run an instrumented pipeline to generate traces." Wire `draw_trace_viewer` into `App::draw()` match on `Screen::TraceViewer { .. }`. Verify all integration tests pass. Run `just ready`.
  - Verify: `cargo test -p assay-tui --test trace_viewer` all pass; `cargo test -p assay-tui` no regressions; `just ready` green
  - Done when: All trace_viewer integration tests pass, all existing TUI tests pass, `just ready` green

## Files Likely Touched

- `crates/assay-tui/src/trace_viewer.rs` (new — types, loading, tree flattening, rendering)
- `crates/assay-tui/src/app.rs` (Screen variant, event handling, draw dispatch, help overlay)
- `crates/assay-tui/src/lib.rs` (pub mod trace_viewer)
- `crates/assay-tui/tests/trace_viewer.rs` (new — integration tests)
