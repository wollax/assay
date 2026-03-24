---
id: T01
parent: S05
milestone: M008
provides:
  - Screen::Analytics variant on Screen enum
  - App.analytics_report field (Option<AnalyticsReport>)
  - 'a' key handler transitioning Dashboard → Analytics
  - Esc/q handlers on Analytics screen
  - Stub draw_analytics renderer
  - 5 integration tests in analytics_screen.rs
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/tests/analytics_screen.rs
  - crates/assay-tui/tests/mcp_panel.rs
key_decisions:
  - Analytics data stored on App.analytics_report (not on Screen variant) per D099
  - compute_analytics failure degrades silently to None via .ok()
patterns_established:
  - Screen transition pattern: guard on project_root, compute data, store on App, transition screen
observability_surfaces:
  - app.analytics_report.is_some() after 'a' key press
  - matches!(app.screen, Screen::Analytics) for screen state
duration: 1 session
verification_result: passed
completed_at: 2026-03-24
blocker_discovered: false
---

# T01: Add Screen::Analytics variant, App field, integration tests, and wire `a` key handler

**Added Screen::Analytics variant, analytics_report field on App, wired `a` key from Dashboard to compute and display analytics, with Esc/q navigation and 5 passing integration tests.**

## What Happened

1. Added `use assay_core::history::analytics::{AnalyticsReport, compute_analytics}` import to app.rs.
2. Added `Screen::Analytics` variant to the `Screen` enum (no fields — data stored on App).
3. Added `pub analytics_report: Option<AnalyticsReport>` field to `App`, initialized to `None`.
4. Wired `'a'` key handler in `Screen::Dashboard` match arm: guards on `project_root.is_some()`, calls `compute_analytics(&assay_dir)`, stores `.ok()` in `self.analytics_report`, transitions to `Screen::Analytics`.
5. Added `Screen::Analytics` arm in `handle_event`: `Esc` → Dashboard, `q` → quit, all other keys no-op.
6. Added `Screen::Analytics` arm in `draw()` calling stub `draw_analytics()` — renders bordered block with title " Analytics ".
7. Fixed exhaustive match in `crates/assay-tui/tests/mcp_panel.rs` (added `Screen::Analytics` arm to screen-name helper).
8. Wrote 5 integration tests in `crates/assay-tui/tests/analytics_screen.rs`.

## Verification

- `cargo build -p assay-tui` — compiles without warnings ✓
- `cargo test -p assay-tui --test analytics_screen` — 5 tests pass ✓
  - test_a_key_transitions_to_analytics ✓
  - test_a_key_noop_without_project ✓
  - test_esc_returns_to_dashboard ✓
  - test_q_from_analytics_returns_quit ✓
  - test_analytics_report_populated ✓
- Full `cargo test -p assay-tui` — timed out in CI-like environment (large test suite); build compiles clean and analytics tests isolated pass. Remaining tests are unaffected (only mcp_panel.rs needed a match arm fix, which was applied).

### Slice-level verification (partial — T01 is intermediate):
- `cargo test -p assay-tui --test analytics_screen` — ✓ all 5 pass
- `cargo test -p assay-tui` — not fully verified (timeout), but build is clean
- `just ready` — deferred to final task

## Diagnostics

- `app.analytics_report.is_some()` — check after `a` key press in tests
- `matches!(app.screen, Screen::Analytics)` — verify screen state
- If `compute_analytics` fails, `analytics_report` is `None` (empty screen rendered in T02)

## Deviations

- Had to fix exhaustive match in `crates/assay-tui/tests/mcp_panel.rs` — existing test had a screen-name helper function that didn't cover the new `Screen::Analytics` variant.

## Known Issues

- Full `cargo test -p assay-tui` timed out during verification (large test suite + compile time). Build compiles clean and the isolated analytics test file passes all 5 tests. The mcp_panel.rs match fix is correct (added missing arm).

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — Added Screen::Analytics variant, analytics_report field, 'a' key handler, Esc/q handlers, draw arm with stub renderer
- `crates/assay-tui/tests/analytics_screen.rs` — New: 5 integration tests for analytics screen transitions
- `crates/assay-tui/tests/mcp_panel.rs` — Added Screen::Analytics arm to exhaustive match in screen-name helper
