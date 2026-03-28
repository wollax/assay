# S02: TUI Trace Viewer — Research

**Date:** 2026-03-28
**Domain:** Ratatui TUI, tracing/telemetry, JSON file parsing
**Confidence:** HIGH

## Summary

The TUI trace viewer reads `.assay/traces/*.json` files written by `JsonFileLayer` (M009/S04) and renders them in a new `Screen::TraceViewer` variant. The JSON format is well-defined (`Vec<SpanData>` with parent-child via `parent_id`), the CLI already has a working parser and tree renderer (`crates/assay-cli/src/commands/traces.rs`), and the TUI has well-established patterns for new full-screen views (Analytics, McpPanel, MilestoneDetail).

The primary coupling surface is the `SpanData` struct in `assay_core::telemetry`. This struct is already `pub`, `Serialize + Deserialize`, and has integration tests proving its round-trip fidelity. The trace viewer can import it directly — no new types needed in `assay-types`.

Risk is medium due to the span tree rendering in a scrollable Ratatui widget (need to flatten a tree into lines with indentation), but the CLI `traces.rs` already has the tree-walking logic. The TUI version ports the same adjacency-map approach into Ratatui `List`/`ListItem` rendering.

## Recommendation

**Port the existing CLI trace parsing and tree logic into a new `trace_viewer.rs` module in assay-tui.** Reuse `SpanData` from `assay_core::telemetry` directly. Follow the established TUI screen pattern: `Screen::TraceViewer` variant, `draw_trace_viewer()` free function, `handle_trace_viewer_event()` method or match arm in `handle_event()`.

Two-level navigation:
1. **Trace list** — top-20 most recent `.json` files sorted by mtime (D180), showing timestamp, root span name, span count, duration.
2. **Span tree** — Enter on a trace flattens the span tree into indented lines with timing; Up/Down navigates; Esc returns to trace list; second Esc returns to Dashboard.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Trace JSON parsing | `assay_core::telemetry::SpanData` + `serde_json::from_str` | Already proven by 6 integration tests in `trace_export.rs`; exact same format |
| Trace file discovery | `std::fs::read_dir` + filter `.json` + sort by mtime | CLI `traces.rs` does this; copy the pattern |
| Span tree adjacency | `HashMap<Option<u64>, Vec<&SpanData>>` | CLI `traces.rs::print_span_tree` uses this exact structure |
| Scrollable list | `ratatui::widgets::List` + `ListState` | Used everywhere in the TUI (Dashboard, MilestoneDetail, McpPanel) |

## Existing Code and Patterns

- `crates/assay-core/src/telemetry.rs` — `SpanData` struct (the trace file schema), `JsonFileLayer` (the writer). SpanData fields: `name`, `target`, `level`, `span_id`, `parent_id`, `start_time`, `end_time`, `duration_ms`, `fields`. This is the **authoritative format** — the viewer must parse exactly this.
- `crates/assay-cli/src/commands/traces.rs` — `load_trace()` parses a JSON file into `Vec<SpanData>`. `print_span_tree()` builds a `HashMap<Option<u64>, Vec<&SpanData>>` adjacency map and recursively renders. Port this tree-flattening logic for TUI rendering.
- `crates/assay-tui/src/app.rs` — `Screen` enum (add `TraceViewer` variant), `App::draw()` match dispatch, `App::handle_event()` match dispatch. Follow D097 (pass individual fields, not `&mut App`), D105 (all `draw_*` accept explicit `area: Rect`), D098 (`..` pattern to avoid borrow-split).
- `crates/assay-tui/src/mcp_panel.rs` — Example of a separate module for a screen: types + draw function + I/O helpers. The trace viewer should follow this pattern with a `trace_viewer.rs` module.
- `crates/assay-tui/tests/analytics_screen.rs` — Test pattern: `TempDir` + fixture project + `App::with_project_root()` + synthetic `KeyEvent`s + assert on `Screen` variant transitions.
- `crates/assay-core/tests/trace_export.rs` — `with_json_layer()` helper creates real trace files via `JsonFileLayer`. Can be used or adapted to create test fixtures for the viewer integration test.

## Constraints

- **D180:** Top-20 most recent traces sorted by mtime descending. Same cap as `assay traces list`.
- **D091:** Loading is synchronous, on navigation transitions only. Load trace files when `t` is pressed and when a trace is selected — not on every frame.
- **D097/D105:** `draw_trace_viewer(frame, area, ...)` takes individual fields, receives `area: Rect` from the top-level layout split.
- **D089:** `App` owns all state; `Screen::TraceViewer` variant holds the view-specific state inline.
- **D107:** Event loop is channel-based (`TuiEvent`), but trace loading is sync (no background thread needed per D091).
- **ratatui 0.30:** Current workspace version. List, ListState, Table, Block, Borders all available.
- **`t` key unbound:** Confirmed no existing `'t'` handler in Dashboard. The help overlay does not list `t` yet — must add it.
- **`SpanData` is `pub`:** Can be imported directly from `assay_core::telemetry`. No need to duplicate types.

## Common Pitfalls

- **Trace file with no root span** — If all spans have a `parent_id` (orphaned children from a crash or concurrent trace), the tree rendering produces nothing visible. Guard: treat spans whose `parent_id` doesn't match any other span's `span_id` as roots. The CLI doesn't guard for this — the TUI should.
- **Large trace files** — A trace with hundreds of spans flattened into a list could be slow to render. Cap the span count displayed (e.g. 500 spans) and show a truncation notice. In practice, typical traces have 10-50 spans, so this is defensive.
- **Borrow-split on Screen variant** — `Screen::TraceViewer` will carry state (traces vec, selected index, expanded trace spans, sub-selected index). Use `..` pattern in `draw()` match and store scrollable state (`ListState`) on `App` as `trace_list_state` (following D098/D099 pattern from MilestoneDetail).
- **Mtime vs filename ordering** — D180 says mtime. The CLI sorts by filename (which has a timestamp prefix). Mtime is more reliable for "most recent" since the filename timestamp is the span start time, not necessarily the file write time. Use `metadata().modified()` for sorting, fall back to filename sort if mtime is unavailable.
- **Empty traces directory** — Either `.assay/traces/` doesn't exist or is empty. Show an informative message ("No traces found. Run an instrumented pipeline to generate traces.") rather than an empty screen.

## Open Risks

- **Concurrent trace writing** — If a pipeline is actively writing a trace while the TUI reads the directory, a partial JSON file could cause a parse error. Mitigation: `JsonFileLayer` uses atomic tempfile+persist (NamedTempFile pattern), so a file either exists completely or doesn't. Risk is minimal.
- **`SpanData.fields` rendering** — The span tree view should show span name + duration. Fields are additional detail. Deciding how much field data to show inline vs on demand is a UX choice. Start with name + duration only; fields can be added in a future iteration.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| ratatui | `blacktop/dotfiles@ratatui-tui` (61 installs) | available — not needed; existing codebase has mature ratatui patterns |
| ratatui | `padparadscho/skills@rs-ratatui-crate` (22 installs) | available — not needed |

No skills needed — the existing TUI codebase provides all necessary patterns.

## Sources

- `crates/assay-core/src/telemetry.rs` — SpanData struct definition and JsonFileLayer implementation (PRIMARY)
- `crates/assay-cli/src/commands/traces.rs` — CLI trace list/show implementation with tree rendering (PORT SOURCE)
- `crates/assay-core/tests/trace_export.rs` — Integration tests proving JsonFileLayer output format (VALIDATION)
- `crates/assay-tui/src/app.rs` — Screen enum, draw dispatch, event handling patterns (TUI PATTERNS)
- `crates/assay-tui/src/mcp_panel.rs` — Separate module pattern for a TUI screen (MODULE PATTERN)
- `crates/assay-tui/tests/analytics_screen.rs` — TUI integration test pattern (TEST PATTERN)
