---
estimated_steps: 5
estimated_files: 2
---

# T01: Add Screen::Analytics variant, App field, integration tests, and wire `a` key handler

**Slice:** S05 — TUI Analytics Screen
**Milestone:** M008

## Description

Add the `Screen::Analytics` variant to the Screen enum, the `analytics_report: Option<AnalyticsReport>` field to App, wire the `a` key handler in the Dashboard event dispatch, add `Esc`/`q` handling for the new screen, add a stub draw arm, and write integration tests. By the end of this task, pressing `a` on Dashboard with a valid project transitions to Analytics and back.

## Steps

1. **Add `Screen::Analytics` variant** to the `Screen` enum in `app.rs`. No fields — analytics data stored on App per D099.

2. **Add `analytics_report` field to `App`** — `pub analytics_report: Option<AnalyticsReport>` initialized to `None` in `with_project_root`. Add `use assay_core::history::analytics::AnalyticsReport;` import.

3. **Wire `a` key handler in Dashboard** — In the `Screen::Dashboard` match arm of `handle_event`, after the `s` (settings) handler: guard on `self.project_root.is_some()`, call `compute_analytics(&assay_dir)`, store `result.ok()` in `self.analytics_report`, transition to `Screen::Analytics`. Import `compute_analytics`.

4. **Add `Screen::Analytics` arm in `handle_event`** — `Esc` returns to `Screen::Dashboard`; `q` returns `true` (quit). All other keys are no-ops. Add arm in `draw()` calling a stub `draw_analytics(frame, content_area, self.analytics_report.as_ref())` function. The stub renders a bordered block with title " Analytics " — the real tables come in T02.

5. **Write integration tests** in `crates/assay-tui/tests/analytics_screen.rs`:
   - `test_a_key_transitions_to_analytics` — create a project fixture with `.assay/` dir, build App, press `a`, assert `Screen::Analytics`
   - `test_a_key_noop_without_project` — `App::with_project_root(None)`, press `a`, assert still `Screen::NoProject`
   - `test_esc_returns_to_dashboard` — press `a` then `Esc`, assert `Screen::Dashboard`
   - `test_q_from_analytics_returns_quit` — press `a` then `q`, assert `handle_event` returns `true`
   - `test_analytics_report_populated` — press `a`, assert `app.analytics_report.is_some()`

## Must-Haves

- [ ] `Screen::Analytics` variant exists on the Screen enum
- [ ] `App.analytics_report: Option<AnalyticsReport>` field exists and initializes to `None`
- [ ] `a` key from Dashboard with project_root transitions to `Screen::Analytics`
- [ ] `a` key is no-op when `project_root` is `None`
- [ ] `Esc` from Analytics returns to Dashboard
- [ ] `q` from Analytics triggers quit
- [ ] All exhaustive match arms compile (draw + handle_event)
- [ ] Integration tests pass

## Verification

- `cargo test -p assay-tui --test analytics_screen` — 5 tests pass
- `cargo test -p assay-tui` — all existing tests still pass
- `cargo build -p assay-tui` — compiles without warnings

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: `app.analytics_report.is_some()` after `a` key press; `matches!(app.screen, Screen::Analytics)` for screen state
- Failure state exposed: `compute_analytics` failure degrades silently to `analytics_report = None` (empty screen rendered in T02)

## Inputs

- `crates/assay-tui/src/app.rs` — existing Screen enum, App struct, handle_event, draw method
- `crates/assay-core/src/history/analytics.rs` — `compute_analytics`, `AnalyticsReport` types (from S04)
- `crates/assay-tui/tests/spec_browser.rs` — `key()` helper pattern for synthetic key events

## Expected Output

- `crates/assay-tui/src/app.rs` — Screen::Analytics variant, App field, key handler, draw arm with stub
- `crates/assay-tui/tests/analytics_screen.rs` — 5 integration tests exercising screen transitions
