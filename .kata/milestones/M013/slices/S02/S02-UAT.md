# S02: TUI Trace Viewer — UAT

**Milestone:** M013
**Written:** 2026-03-28

## UAT Type

- UAT mode: human-experience
- Why this mode is sufficient: Integration tests prove all structural contracts (screen transitions, navigation, span tree building, empty state, orphan spans). What remains is visual rendering quality (layout aesthetics, color/highlight visibility, scrolling UX with real trace data) which requires a human with a running TUI.

## Preconditions

1. Build the TUI binary: `cargo build -p assay-tui`
2. Generate at least one trace by running a gate or pipeline: `cargo run -p assay-cli -- gate run` in a project with a valid `gates.toml`
3. Alternatively, copy a fixture trace file to `.assay/traces/`: `cp crates/assay-tui/tests/fixtures/*.json .assay/traces/` (if fixture files exist) or run `cargo run -p assay-cli -- gate run` in the test workspace
4. Launch the TUI: `cargo run -p assay-tui` from a project root

## Smoke Test

From the Dashboard, press `t`. The screen should change to show a list of traces with timestamps and span names. If no traces exist, an informative message should appear.

## Test Cases

### 1. Trace list opens from Dashboard

1. Launch TUI: `cargo run -p assay-tui`
2. Ensure a Dashboard is showing (project with milestones loaded, or just the empty dashboard)
3. Press `t`
4. **Expected:** Screen transitions to "Traces" view. If `.assay/traces/` has files: a list of entries showing timestamp, root span name, span count, and duration_ms. If empty: centered message "No traces found. Run an instrumented pipeline to generate traces."

### 2. Navigation in trace list

1. Open trace viewer (press `t`)
2. Press `Down` arrow
3. Press `Up` arrow
4. **Expected:** Selection highlight moves between entries. No crash. Single entry: selection stays on the only item.

### 3. Span tree expansion and collapse

1. Open trace viewer with at least one trace present
2. Select a trace entry with `Down`/`Up`
3. Press `Enter`
4. **Expected:** Screen shows a span tree with the root span and child spans indented (2 spaces per depth level). Each line shows span name and duration. Title block shows the trace root span name.
5. Press `Esc`
6. **Expected:** Returns to trace list (not Dashboard).

### 4. Return to Dashboard

1. Open trace viewer (press `t`)
2. Press `Esc` (from trace list, not span tree)
3. **Expected:** Returns to Dashboard screen.

### 5. Empty state message

1. Ensure `.assay/traces/` directory is empty or does not exist
2. Press `t`
3. **Expected:** Trace viewer shows "No traces found. Run an instrumented pipeline to generate traces." (centered or as a paragraph)

### 6. Help overlay includes `t` key

1. Press `?` or `h` to open the help overlay (if available) from Dashboard
2. **Expected:** `t` key listed under the Dashboard navigation section as "Trace viewer" or similar

## Edge Cases

### Orphan span handling

1. Manually create a trace JSON file in `.assay/traces/` with a span whose `parent_id` references a non-existent `span_id`
2. Open trace viewer and navigate to that trace
3. Press `Enter` to expand
4. **Expected:** Orphan span appears as an additional root at depth 0 (no crash, no blank screen)

### Large trace (many spans)

1. Run a multi-session pipeline to generate a trace with many spans
2. Open trace viewer and expand that trace
3. Press `Down` repeatedly to scroll through the span tree
4. **Expected:** Navigation remains responsive; no truncation without indication

### Many trace files (>20)

1. Generate or copy more than 20 trace files to `.assay/traces/`
2. Open trace viewer
3. **Expected:** List shows at most 20 entries, sorted newest first

## Failure Signals

- Pressing `t` causes a crash or panic → screen transition broken
- Trace list shows no entries when `.assay/traces/` has valid JSON files → load_traces() or SpanData parsing failing
- Pressing `Enter` on a trace shows a blank span tree → flatten_span_tree() output empty
- Pressing `Esc` from span tree exits to Dashboard instead of trace list → Esc chain broken
- `tracing::warn!` messages in stderr about unreadable files when files are valid → parse/IO regression

## Requirements Proved By This UAT

- R066 (TUI trace viewer) — UAT proves: `t` key opens trace viewer, span tree navigable, Esc closes, visual rendering quality acceptable for daily use

## Not Proven By This UAT

- Live trace refresh while viewer is open (deferred — requires re-entry via Esc + `t`)
- Rendering with very wide terminal windows (edge case layout)
- Rendering when span names contain non-ASCII characters
- Performance with traces containing hundreds of spans (structural tests prove correctness; performance is UAT-only)
- OTel metrics integration (R067 — separate S03 work)

## Notes for Tester

- Traces are written to `.assay/traces/` only when running `assay gate run`, `assay run`, or `assay context` subcommands (not when running `assay traces` itself — self-tracing disabled by design per R063)
- If no traces exist, the easiest way to generate one is: `cargo run -p assay-cli -- gate run` in any project with a `gates.toml`
- The TUI help overlay (if accessible) should list `t` as a navigation key; verify it's present
- Selection highlight uses bold+reversed style — verify it's clearly visible in your terminal color scheme
