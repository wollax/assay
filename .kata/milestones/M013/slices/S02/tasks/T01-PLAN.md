---
estimated_steps: 5
estimated_files: 4
---

# T01: Integration tests and trace_viewer module scaffold

**Slice:** S02 — TUI Trace Viewer
**Milestone:** M013

## Description

Create the `trace_viewer` module with core data types and logic (`TraceEntry`, `SpanLine`, `load_traces()`, `flatten_span_tree()`), and the integration test file that exercises screen transitions, trace loading, orphan-span handling, and empty-state behavior. Tests for screen transitions will compile but may fail until T02 wires the Screen variant — that's expected. The module's standalone functions (`load_traces`, `flatten_span_tree`) should be fully testable and passing.

## Steps

1. Add `pub mod trace_viewer;` to `crates/assay-tui/src/lib.rs`.
2. Create `crates/assay-tui/src/trace_viewer.rs` with:
   - `TraceEntry { id: String, timestamp: String, root_span_name: String, span_count: usize, duration_ms: Option<f64> }` — parsed from a trace JSON file.
   - `SpanLine { depth: usize, name: String, duration_ms: Option<f64> }` — one flattened line of a span tree for rendering.
   - `load_traces(assay_dir: &Path) -> Vec<TraceEntry>` — reads `.assay/traces/*.json`, parses each via `SpanData`, sorts by mtime descending, caps at 20. Skips unreadable files with `tracing::warn!`. Returns empty vec if dir doesn't exist.
   - `flatten_span_tree(spans: &[SpanData]) -> Vec<SpanLine>` — builds `HashMap<Option<u64>, Vec<&SpanData>>` adjacency map. Orphan guard: spans whose `parent_id` doesn't match any `span_id` in the set are treated as additional roots (collected alongside `parent_id: None` spans). Recursively flattens with depth tracking. Sort children by `start_time`.
   - Unit tests inside the module: `test_load_traces_empty_dir`, `test_load_traces_sorts_by_mtime`, `test_load_traces_caps_at_20`, `test_flatten_span_tree_basic`, `test_flatten_span_tree_orphan_roots`.
3. Create `crates/assay-tui/tests/trace_viewer.rs` with integration tests:
   - Helper: `key(code) -> KeyEvent`, `setup_project(tmp) -> PathBuf` (creates `.assay/milestones/` + minimal milestone TOML + `.assay/traces/` with fixture JSON files).
   - Helper: `make_span(name, span_id, parent_id, start_time, duration_ms) -> SpanData` and `write_trace_file(dir, id, spans)` — same pattern as CLI traces tests.
   - `test_t_key_transitions_to_trace_viewer` — presses `t` from Dashboard, asserts `Screen::TraceViewer { .. }`.
   - `test_esc_from_trace_list_returns_to_dashboard` — `t` → Esc, asserts `Screen::Dashboard`.
   - `test_empty_traces_dir_shows_trace_viewer` — setup project without traces dir, press `t`, assert `Screen::TraceViewer` with empty traces vec.
4. Add `assay_core::telemetry::SpanData` import in trace_viewer.rs (the type is already pub).
5. Verify: `cargo test -p assay-tui --lib` passes for trace_viewer unit tests. Integration tests may not compile until T02 adds the Screen variant — that's acceptable for T01.

## Must-Haves

- [ ] `trace_viewer.rs` module exists with `TraceEntry`, `SpanLine`, `load_traces()`, `flatten_span_tree()`
- [ ] `load_traces()` reads `.assay/traces/*.json`, sorts by mtime desc, caps at 20, skips bad files
- [ ] `flatten_span_tree()` handles orphan spans (parent_id referencing non-existent span_id)
- [ ] Unit tests for `load_traces` and `flatten_span_tree` pass
- [ ] Integration test file `trace_viewer.rs` exists with screen transition tests

## Verification

- `cargo test -p assay-tui --lib` — trace_viewer unit tests pass
- Integration test file compiles (may have some tests that fail until T02)

## Observability Impact

- Signals added/changed: `tracing::warn!` on unreadable trace files in `load_traces()`
- How a future agent inspects this: call `load_traces()` directly in a test to verify trace discovery; `flatten_span_tree()` output is a plain Vec
- Failure state exposed: parse errors result in skipped entries (no panic, no empty screen)

## Inputs

- `crates/assay-core/src/telemetry.rs` — `SpanData` struct definition (the JSON schema)
- `crates/assay-cli/src/commands/traces.rs` — reference implementation for `load_trace()` and `print_span_tree()` adjacency-map pattern
- `crates/assay-tui/tests/analytics_screen.rs` — integration test pattern (`TempDir`, `setup_project`, `key()` helper)

## Expected Output

- `crates/assay-tui/src/trace_viewer.rs` — module with types and logic, unit tests passing
- `crates/assay-tui/src/lib.rs` — updated with `pub mod trace_viewer`
- `crates/assay-tui/tests/trace_viewer.rs` — integration test file with screen transition tests
