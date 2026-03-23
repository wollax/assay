---
id: S05
milestone: M008
status: ready
---

# S05: TUI analytics screen — Context

## Goal

Deliver a full-screen TUI analytics view (`a` key from Dashboard) showing a color-coded gate failure frequency table and a milestone velocity table, using `compute_analytics()` from S04.

## Why this Slice

S05 is the TUI rendering surface for the analytics engine built in S04. It makes R059 (gate history analytics) visible through the primary product surface (the TUI, per D068). S05 depends on S04's `compute_analytics()` and `AnalyticsReport` types — without them, there's nothing to render.

## Scope

### In Scope

- New `Screen::Analytics` variant in assay-tui's Screen enum
- `a` key handler from Dashboard that calls `compute_analytics()` synchronously and transitions to `Screen::Analytics`
- `draw_analytics(frame, area, report)` free function following D097/D105 patterns
- Vertical split layout: failure frequency table (top half) + milestone velocity table (bottom half)
- Failure frequency table: columns for criterion name, spec name, fail count, total runs — sorted by fail count descending, row background color intensity based on failure rate (red tones = high failure, dim = low)
- Milestone velocity table: columns for milestone name, status, chunks completed/total, days elapsed, velocity (chunks/day)
- Esc/q returns to Dashboard
- Friendly empty state message when no history exists: "No gate history yet — run assay gate run to get started"
- Integration tests driving synthetic key events (same pattern as M006/M007 TUI tests)

### Out of Scope

- Interactive drill-down into individual criterion failure details — future slice
- Time-range filtering within the TUI (CLI has --limit/--since; TUI shows default view)
- ASCII charts or sparklines — tables only for S05
- Live refresh / auto-reload of analytics data (loaded once on screen entry)
- Export from TUI to file

## Constraints

- Full-screen view, not a popup — consistent with D102 (Settings is full-screen)
- `draw_analytics` accepts `area: Rect` per D105 (global layout split once in draw())
- Data loaded synchronously on `a` key press per D091 — `compute_analytics()` should be fast (<100ms)
- Analytics data stored on App-level fields per D099 pattern (e.g. `App.analytics_report: Option<AnalyticsReport>`)
- Color intensity for failure rows using Ratatui `Style::bg()` with graduated red — no external color library

## Integration Points

### Consumes

- `assay-core::history::analytics::compute_analytics(assay_dir, specs_dir, options)` — returns `AnalyticsReport`
- `AnalyticsReport { failure_frequency: Vec<FailureFrequency>, milestone_velocity: Vec<MilestoneVelocity> }` — from S04
- `FailureFrequency { criterion_name, spec_name, fail_count, total_runs }` — row data for top table
- `MilestoneVelocity { milestone_slug, milestone_name, status, chunks_completed, total_chunks, days_elapsed, velocity }` — row data for bottom table
- Existing TUI patterns: Screen enum, App struct, draw dispatch, D097/D099/D105

### Produces

- `Screen::Analytics` variant
- `draw_analytics(frame, area, report)` free function
- `App.analytics_report: Option<AnalyticsReport>` field
- `a` key handler in Dashboard's handle_event
- Integration tests in `tests/analytics.rs`

## Open Questions

- None — all behavioral decisions captured during discuss.
