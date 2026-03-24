---
estimated_steps: 4
estimated_files: 2
---

# T02: Implement draw_analytics with failure frequency and velocity tables, update help overlay

**Slice:** S05 — TUI Analytics Screen
**Milestone:** M008

## Description

Replace the stub `draw_analytics` with a full implementation rendering two ratatui Tables: a failure frequency heatmap-style table with color-coded failure rates, and a milestone velocity summary table. Handle the empty-report case with a centered message. Update the help overlay to mention the `a` key. Add a smoke test proving data-driven rendering doesn't panic.

## Steps

1. **Implement `draw_analytics` function body** — Accept `frame`, `area: Rect`, `report: Option<&AnalyticsReport>`. If `report` is `None` or both vecs are empty, render a centered "No analytics data available" Paragraph in a bordered block and return early. Otherwise, use `Layout::vertical` to split area into: title block, failure frequency table area, velocity table area, and a hint line area.

2. **Render failure frequency table** — Block title "Gate Failure Frequency". Columns: Spec (`Constraint::Length(20)`), Criterion (`Constraint::Fill(1)`), Fails (`Length(6)`), Runs (`Length(6)`), Rate (`Length(8)`), Enforcement (`Length(10)`). For each `FailureFrequency` entry: compute `rate = fail_count as f64 / total_runs as f64 * 100.0` (guard `total_runs == 0`); format rate as `{:.1}%`; color the Rate cell red if rate > 50.0, yellow if rate > 0.0, green if rate == 0.0. Enforcement column shows "Required" or "Advisory".

3. **Render milestone velocity table** — Block title "Milestone Velocity". Columns: Milestone (`Constraint::Fill(1)`), Chunks (`Length(10)`), Days (`Length(8)`), Velocity (`Length(10)`). For each `MilestoneVelocity` entry: format chunks as `{completed}/{total}`, days as `{:.1}`, velocity as `{:.1}/day`. Hint line at bottom: "Esc back  q quit".

4. **Update help overlay** — Add a row after the `n → New milestone (wizard)` row: `Cell::from("  a")` / `Cell::from("Analytics")`. Increment the `h` constant for popup height by 1 to accommodate the new row. Add a smoke test in `analytics_screen.rs` that constructs an `App` with `analytics_report = Some(report)` containing synthetic data and asserts the screen variant is `Screen::Analytics` (proves the data path doesn't panic during state setup).

## Must-Haves

- [ ] `draw_analytics` renders failure frequency table with color-coded rates
- [ ] `draw_analytics` renders milestone velocity table with formatted numbers
- [ ] Empty/None report shows "No analytics data available" message
- [ ] Rate color thresholds match S04 CLI: red >50%, yellow >0%, green 0%
- [ ] Help overlay includes `a → Analytics` row
- [ ] `just ready` passes (fmt, lint, test, deny)

## Verification

- `cargo test -p assay-tui --test analytics_screen` — all tests pass (including new smoke test)
- `cargo test -p assay-tui` — all TUI tests pass
- `just ready` — full workspace green

## Observability Impact

- Signals added/changed: None — pure rendering function
- How a future agent inspects this: Visual inspection of TUI; test assertions on screen state
- Failure state exposed: Empty report renders "No analytics data available" instead of blank/panic

## Inputs

- `crates/assay-tui/src/app.rs` — stub `draw_analytics` from T01 to be replaced
- `crates/assay-core/src/history/analytics.rs` — `AnalyticsReport`, `FailureFrequency`, `MilestoneVelocity` type fields
- `crates/assay-cli/src/commands/history.rs` — reference for rate calculation and formatting (port to ratatui)
- T01 integration tests — existing tests continue to pass

## Expected Output

- `crates/assay-tui/src/app.rs` — complete `draw_analytics` function with two tables and color coding; help overlay with `a` row
- `crates/assay-tui/tests/analytics_screen.rs` — additional smoke test for data-driven rendering
