# S05: TUI Analytics Screen

**Goal:** Pressing `a` from the TUI Dashboard opens a full-screen analytics screen showing gate failure frequency (heatmap-style table) and milestone velocity summary, sourced from `compute_analytics()` delivered by S04.
**Demo:** User presses `a` on Dashboard → analytics screen shows failure frequency table with color-coded rates and milestone velocity table; `Esc`/`q` returns to Dashboard. Empty data shows "No analytics data" message.

## Must-Haves

- `Screen::Analytics` variant added to the `Screen` enum
- `App.analytics_report: Option<AnalyticsReport>` field stores loaded analytics data (D099)
- `a` key from Dashboard calls `compute_analytics(&assay_dir)` and transitions to `Screen::Analytics`
- `draw_analytics(frame, area, report)` free function renders two tables: failure frequency + milestone velocity
- Failure frequency table color-codes rates: red >50%, yellow >0%, green 0% (matching S04 CLI thresholds)
- Milestone velocity table shows slug, chunks completed/total, days elapsed, chunks/day with `{:.1}` formatting
- Empty report renders a centered "No analytics data available" message
- `Esc`/`q` from Analytics returns to Dashboard
- Help overlay updated with `a` key entry for Analytics
- `a` is a no-op when `project_root` is `None` (same guard as `r` and `n`)
- Integration tests prove screen transitions and data-driven rendering

## Proof Level

- This slice proves: integration (synthetic key events drive screen transitions; analytics data renders correctly)
- Real runtime required: no (tests use synthetic data and App state mutations)
- Human/UAT required: no (visual verification is nice-to-have, not required)

## Verification

- `cargo test -p assay-tui --test analytics_screen` — integration tests covering:
  - `a` key from Dashboard transitions to `Screen::Analytics`
  - `a` is no-op when `project_root` is `None`
  - `Esc` from Analytics returns to Dashboard
  - `q` from Analytics returns to Dashboard (quit signal)
  - Analytics report data is stored on App after `a` key
  - Empty analytics report doesn't panic
- `cargo test -p assay-tui` — all existing TUI tests still pass
- `just ready` — full workspace checks pass

## Observability / Diagnostics

- Runtime signals: None (synchronous data load on `a` key press; no background threads)
- Inspection surfaces: `App.analytics_report` field inspectable in tests; `assay history analytics --json` for data verification
- Failure visibility: If `compute_analytics` fails, `a` key is a no-op (error degrades to None via `.ok()`)
- Redaction constraints: None

## Integration Closure

- Upstream surfaces consumed: `assay_core::history::analytics::{compute_analytics, AnalyticsReport, FailureFrequency, MilestoneVelocity}` from S04
- New wiring introduced in this slice: `Screen::Analytics` variant + `draw_analytics` renderer + `a` key handler in Dashboard + help overlay entry
- What remains before the milestone is truly usable end-to-end: nothing — S05 is the final slice in M008

## Tasks

- [ ] **T01: Add Screen::Analytics variant, App field, integration tests, and wire `a` key handler** `est:25m`
  - Why: Establishes the Screen variant, App-level analytics storage, key handler, and test contract. Tests start passing immediately since the handler + screen variant + draw match arm are all wired in this task.
  - Files: `crates/assay-tui/src/app.rs`, `crates/assay-tui/tests/analytics_screen.rs`
  - Do: Add `Screen::Analytics` variant (no fields — D099). Add `App.analytics_report: Option<AnalyticsReport>`. Wire `a` key in Dashboard (guard on `project_root`, call `compute_analytics`, store result, transition screen). Add `Esc`/`q` handling in `Screen::Analytics` match arm. Add stub `draw_analytics` arm in `draw()` that renders a placeholder block. Write integration tests using synthetic key events and `App::with_project_root`. Follow D097/D098/D105 patterns exactly.
  - Verify: `cargo test -p assay-tui --test analytics_screen` passes; `cargo test -p assay-tui` all pass
  - Done when: `a` from Dashboard transitions to Analytics, `Esc`/`q` returns, tests pass, `analytics_report` populated

- [ ] **T02: Implement draw_analytics with failure frequency and velocity tables, update help overlay** `est:25m`
  - Why: Renders the actual analytics content — failure heatmap table and velocity summary — completing the user-visible feature. Updates help overlay so users discover the `a` key.
  - Files: `crates/assay-tui/src/app.rs`, `crates/assay-tui/tests/analytics_screen.rs`
  - Do: Replace the stub `draw_analytics` with a full implementation: bordered block titled "Analytics", vertical layout split for two tables + hints. Failure frequency table: columns Spec, Criterion, Fails, Runs, Rate, Enforcement; rate color-coded (red >50%, yellow >0%, green 0%). Velocity table: columns Milestone, Chunks, Days, Velocity. Empty report → centered "No analytics data available" paragraph. Add `a → Analytics` row to the Dashboard section of `draw_help_overlay`. Add a smoke test that constructs an App with an AnalyticsReport and verifies no panic on screen variant.
  - Verify: `cargo test -p assay-tui --test analytics_screen` passes; `just ready` passes
  - Done when: `draw_analytics` renders both tables with correct color coding; help overlay mentions `a`; `just ready` green

## Files Likely Touched

- `crates/assay-tui/src/app.rs` — Screen variant, App field, key handler, draw_analytics, help overlay
- `crates/assay-tui/tests/analytics_screen.rs` — Integration tests (new file)
