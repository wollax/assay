---
estimated_steps: 5
estimated_files: 3
---

# T03: Render trace list and span tree

**Slice:** S02 — TUI Trace Viewer
**Milestone:** M013

## Description

Implement `draw_trace_viewer()` — the visual rendering of both the trace list and span tree views. Wire it into `App::draw()`. Handle the empty-state message. Verify all tests pass and `just ready` is green.

## Steps

1. Implement `draw_trace_viewer()` in `trace_viewer.rs` following D097/D105 pattern:
   ```rust
   pub fn draw_trace_viewer(
       frame: &mut ratatui::Frame,
       area: Rect,
       traces: &[TraceEntry],
       trace_list_state: &mut ListState,
       selected_trace: Option<usize>,
       span_lines: &[SpanLine],
       span_list_state: &mut ListState,
   )
   ```
   - **Empty state** (`traces.is_empty()`): render a centered `Paragraph` with "No traces found. Run an instrumented pipeline to generate traces." inside a bordered block titled "Traces".
   - **Trace list view** (`selected_trace.is_none()`): bordered block titled "Traces (t)". Each `ListItem` shows: `"{timestamp}  {root_span_name}  spans:{span_count}  {duration}"` where duration formats as `"{ms:.1}ms"` or `"—"` if None. Highlight selected item with a distinct style (e.g. `Style::default().fg(Color::Cyan).bold()`). Render as stateful list with `trace_list_state`.
   - **Span tree view** (`selected_trace.is_some()`): bordered block titled "Trace: {root_span_name}". Each `ListItem` shows: `"{"  ".repeat(depth)}{name}  ({duration_ms:.1}ms)"` or `"{"  ".repeat(depth)}{name}"` if duration is None. Highlight selected with same style. Render as stateful list with `span_list_state`.

2. Wire `draw_trace_viewer` into `App::draw()` match on `Screen::TraceViewer`:
   - Use the `..` pattern (D098) to avoid borrow-split issues.
   - Pass individual fields from the variant to `draw_trace_viewer()` — but since we need `&mut ListState`, use a temporary extraction pattern: match the variant, get references to the immutable fields and mutable list states.
   - Alternative: store `trace_list_state` and `span_list_state` as `App`-level fields (like `detail_list_state`) if borrow-split is problematic. Follow whichever pattern compiles cleanly.

3. Add integration test `test_trace_viewer_with_traces_loads_entries`:
   - Setup project with 2 trace fixture files in `.assay/traces/`.
   - Press `t`, assert `Screen::TraceViewer` with `traces.len() == 2`.
   - Verify traces are sorted by mtime descending (the more recently modified file is first).

4. Add integration test `test_enter_expands_span_tree_and_esc_returns`:
   - Setup project with a trace containing 3 spans (root + 2 children).
   - Press `t`, then `Enter` on the first trace.
   - Assert `selected_trace.is_some()` and `span_lines.len() == 3`.
   - Press `Esc`, assert `selected_trace.is_none()`.

5. Run `cargo test -p assay-tui` (all tests), then `just ready` for full workspace verification.

## Must-Haves

- [ ] `draw_trace_viewer()` renders trace list, span tree, and empty state
- [ ] `draw_trace_viewer()` wired into `App::draw()` for `Screen::TraceViewer`
- [ ] Empty state renders informative message (not blank screen)
- [ ] Integration test proves trace loading populates entries correctly
- [ ] Integration test proves Enter→span tree→Esc→trace list navigation
- [ ] `just ready` green

## Verification

- `cargo test -p assay-tui --test trace_viewer` — all tests pass
- `cargo test -p assay-tui` — no regressions
- `just ready` — green

## Observability Impact

- Signals added/changed: None (rendering is pure output)
- How a future agent inspects this: pattern-match on `Screen::TraceViewer` fields to inspect traces/span_lines in tests
- Failure state exposed: empty state message visible when no traces exist

## Inputs

- `crates/assay-tui/src/trace_viewer.rs` — types and logic from T01, `load_trace_spans()` from T02
- `crates/assay-tui/src/app.rs` — Screen::TraceViewer variant and event handling from T02
- `crates/assay-tui/tests/trace_viewer.rs` — test harness from T01

## Expected Output

- `crates/assay-tui/src/trace_viewer.rs` — `draw_trace_viewer()` implemented
- `crates/assay-tui/src/app.rs` — draw dispatch wired for TraceViewer
- `crates/assay-tui/tests/trace_viewer.rs` — all integration tests passing
- `just ready` green with all workspace tests passing
