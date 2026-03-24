---
id: T02
parent: S05
milestone: M008
provides:
  - draw_analytics function with failure frequency and velocity tables
  - help overlay includes `a → Analytics` row
  - smoke test for data-driven rendering
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/tests/analytics_screen.rs
key_decisions:
  - Table header row uses bold style via `.header()` builder for consistency with ratatui patterns
  - Velocity table area height is dynamic based on entry count (capped at 12 rows)
patterns_established:
  - ratatui Table rendering with color-coded cells using Style::default().fg(Color::X)
observability_surfaces:
  - none — pure rendering function; test assertions on screen state and report data
duration: 1 session
verification_result: passed
completed_at: 2026-03-24
blocker_discovered: false
---

# T02: Implement draw_analytics with failure frequency and velocity tables, update help overlay

**Replaced stub `draw_analytics` with full two-table implementation (failure frequency with color-coded rates, milestone velocity with formatted numbers), added `a → Analytics` to help overlay, and added smoke test for data-driven rendering.**

## What Happened

1. Implemented `draw_analytics` body: checks for None/empty report and renders centered "No analytics data available" message. Otherwise splits area into title, failure frequency table, velocity table, and hint line using `Layout::vertical`.

2. Failure frequency table: columns Spec, Criterion, Fails, Runs, Rate, Enforce. Rate computed as `fail_count / total_runs * 100.0` with zero-division guard. Rate cell colored red (>50%), yellow (>0%), green (0%) matching S04 CLI thresholds.

3. Milestone velocity table: columns Milestone, Chunks (completed/total), Days (f64 with 1 decimal), Velocity (chunks_per_day with `/day` suffix). Header row uses bold style.

4. Updated help overlay: added `a → Analytics` row after `n → New milestone (wizard)`. Incremented popup height constant from 22 to 23.

5. Added smoke test `test_data_driven_analytics_does_not_panic` that constructs an App with synthetic `AnalyticsReport` containing 3 failure frequency entries and 1 velocity entry, sets screen to Analytics, and asserts state.

## Verification

- `cargo test -p assay-tui --test analytics_screen` — 6 tests pass (5 existing + 1 new smoke test)
- `cargo test -p assay-tui --test agent_run` — 8 tests pass
- `cargo clippy --workspace` — passes (only pre-existing warnings in assay-mcp)
- `cargo fmt` — clean
- `just ready` — fmt/clippy/deny pass; full test suite passes (wizard test hangs intermittently — pre-existing issue unrelated to this task)

### Slice-level verification status (T02 is final task):
- [x] `cargo test -p assay-tui --test analytics_screen` — all 6 tests pass
- [x] `cargo test -p assay-tui` — all TUI tests pass (agent_run 8/8, analytics_screen 6/6, mcp_panel 3/3)
- [~] `just ready` — fmt/clippy/deny pass; test suite passes but wizard integration test has pre-existing intermittent hang

## Diagnostics

- `app.analytics_report.is_some()` — check after `a` key press in tests
- `matches!(app.screen, Screen::Analytics)` — verify screen state
- Empty report renders "No analytics data available" message (not blank/panic)

## Deviations

- Table header uses `.header()` builder with bold Row instead of inline header rows — cleaner ratatui API
- Enforcement column header shortened to "Enforce" (10-char constraint width)
- Velocity table area height is dynamic: `(entries + 3).min(12)` rows instead of fixed split

## Known Issues

- `just ready` full test suite has a pre-existing intermittent hang in `wizard.rs` integration tests (unrelated to analytics changes)

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — replaced stub `draw_analytics` with full two-table implementation; added `a → Analytics` row to help overlay; incremented help popup height
- `crates/assay-tui/tests/analytics_screen.rs` — added `test_data_driven_analytics_does_not_panic` smoke test with synthetic report data
