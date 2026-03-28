---
estimated_steps: 5
estimated_files: 3
---

# T02: Screen::TraceViewer variant and event handling

**Slice:** S02 — TUI Trace Viewer
**Milestone:** M013

## Description

Add the `Screen::TraceViewer` variant to the `Screen` enum and wire the `t` key handler, navigation events (Up/Down/Enter/Esc), and help overlay update. After this task, the TUI responds to `t` from Dashboard by loading traces and transitioning to the TraceViewer screen, with full keyboard navigation between trace list and span tree views.

## Steps

1. Add `Screen::TraceViewer` variant to the `Screen` enum in `app.rs`:
   ```
   TraceViewer {
       traces: Vec<TraceEntry>,
       trace_list_state: ListState,
       // None = viewing trace list; Some(idx) = viewing span tree for traces[idx]
       selected_trace: Option<usize>,
       span_lines: Vec<SpanLine>,
       span_list_state: ListState,
   }
   ```
   Import `TraceEntry` and `SpanLine` from `crate::trace_viewer`.
2. Add `t` key handler in the `Screen::Dashboard` match arm (after existing `s` handler):
   - Call `load_traces(&assay_dir)` (guard on `self.project_root`).
   - Initialize `trace_list_state` with `select(Some(0))` if traces non-empty, else `select(None)`.
   - Transition to `Screen::TraceViewer { traces, trace_list_state, selected_trace: None, span_lines: vec![], span_list_state: ListState::default() }`.
3. Add `Screen::TraceViewer { .. }` match arm in `handle_event()`:
   - When `selected_trace` is `None` (trace list mode):
     - `Up/Down`: navigate `trace_list_state` within `traces.len()` bounds.
     - `Enter`: if a trace is selected, load its full JSON via `load_trace_spans()` (new helper in trace_viewer.rs that reads the file and returns `Vec<SpanData>`), call `flatten_span_tree()`, set `selected_trace = Some(idx)`, initialize `span_list_state` with `select(Some(0))`.
     - `Esc`: return to `Screen::Dashboard`.
     - `q`: return true (quit).
     - `/`: open slash overlay.
   - When `selected_trace` is `Some(_)` (span tree mode):
     - `Up/Down`: navigate `span_list_state` within `span_lines.len()` bounds.
     - `Esc`: set `selected_trace = None`, clear `span_lines` and `span_list_state`.
     - `q`: return true (quit).
4. Add a `load_trace_spans(assay_dir: &Path, trace_id: &str) -> Vec<SpanData>` helper in `trace_viewer.rs` that reads a single trace JSON file and returns the parsed spans (returning empty vec on error with `tracing::warn!`).
5. Add `t` key entry to the help overlay in `draw_help_overlay()` under the Dashboard section: `Row::new(vec![Cell::from("  t"), Cell::from("Traces")])`. Increment the help overlay height `h` by 1.

## Must-Haves

- [ ] `Screen::TraceViewer` variant exists with `traces`, `trace_list_state`, `selected_trace`, `span_lines`, `span_list_state` fields
- [ ] `t` from Dashboard loads traces and transitions to `Screen::TraceViewer`
- [ ] Up/Down navigates trace list; Enter expands span tree; Esc chain works (span→list→Dashboard)
- [ ] Help overlay lists `t` key under Dashboard section
- [ ] Integration tests `test_t_key_transitions_to_trace_viewer` and `test_esc_from_trace_list_returns_to_dashboard` pass

## Verification

- `cargo test -p assay-tui --test trace_viewer` — screen transition tests pass
- `cargo test -p assay-tui` — no regressions in existing tests

## Observability Impact

- Signals added/changed: None beyond T01's tracing::warn on load failures
- How a future agent inspects this: pattern-match on `app.screen` to check `Screen::TraceViewer` fields in tests
- Failure state exposed: empty traces vec is a valid state (not an error); load failure produces empty vec

## Inputs

- `crates/assay-tui/src/trace_viewer.rs` — `TraceEntry`, `SpanLine`, `load_traces()`, `flatten_span_tree()` from T01
- `crates/assay-tui/src/app.rs` — existing Screen enum and handle_event() patterns
- `crates/assay-tui/tests/trace_viewer.rs` — integration tests from T01

## Expected Output

- `crates/assay-tui/src/app.rs` — `Screen::TraceViewer` variant, `t` handler, navigation, help overlay update
- `crates/assay-tui/src/trace_viewer.rs` — `load_trace_spans()` helper added
- All trace_viewer integration tests passing
