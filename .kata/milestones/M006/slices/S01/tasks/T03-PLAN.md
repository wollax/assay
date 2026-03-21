---
estimated_steps: 6
estimated_files: 2
---

# T03: Navigation, empty-state guard, and unit tests

**Slice:** S01 — App Scaffold, Dashboard, and Binary Fix
**Milestone:** M006

## Description

Completes the slice's interactive behavior: ↑↓ arrow keys with wrapping, `Enter` transitioning to `Screen::MilestoneDetail` stub, `Esc` returning to Dashboard, empty-list guard preventing `ListState` panic. Then creates `crates/assay-tui/tests/app_state.rs` with unit tests that exercise all state transitions by constructing `App` directly and calling `handle_event` with synthetic events — no terminal required. Finishes with `just ready`.

## Steps

1. In `handle_event`, implement `KeyCode::Down`: if `app.milestones.is_empty()`, do nothing; otherwise, increment selection with wrap: `let new = match app.list_state.selected() { None | Some(n) if n >= app.milestones.len() - 1 => 0, Some(n) => n + 1, }; app.list_state.select(Some(new));`

2. Implement `KeyCode::Up`: same pattern, decrement with wrap: `let new = match app.list_state.selected() { None | Some(0) => app.milestones.len().saturating_sub(1), Some(n) => n - 1, }; app.list_state.select(Some(new));`

3. Implement `KeyCode::Enter` in Dashboard screen arm: if `app.milestones.is_not_empty()` and a selection exists, transition `app.screen = Screen::MilestoneDetail`. Wire the `MilestoneDetail` screen arm in `draw` to render `Paragraph::new("Milestone detail — coming in S03")`.

4. Implement `KeyCode::Esc`: set `app.screen = Screen::Dashboard` (return to dashboard from any non-dashboard screen).

5. In `draw_dashboard`: add empty-milestones guard before building the `List`. If `app.milestones.is_empty()`, render `Paragraph::new("No milestones — press n to create one")` centered inside the block and return early without building the `List` or calling `render_stateful_widget`. This is the `ListState` panic prevention pattern from the research.

6. Create `crates/assay-tui/tests/app_state.rs`:
   ```rust
   use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
   use assay_tui::{App, Screen, handle_event};  // or inline via pub fn / test helper
   ```
   Tests (use a helper `fn key(code: KeyCode) -> Event` to build synthetic events):
   - `navigate_down_increments_selection`: build `App` with 3 fake milestones (use `Milestone` fixtures constructed directly, not from disk), `list_state.select(Some(0))`; call `handle_event` with `Down`; assert `list_state.selected() == Some(1)`
   - `navigate_down_wraps_to_first`: selection at last index; `Down`; assert selection wraps to `0`
   - `navigate_up_wraps_to_last`: selection at `0`; `Up`; assert selection wraps to last index
   - `quit_returns_false_from_dashboard`: call `handle_event` with `Char('q')`; assert returns `false`
   - `enter_on_dashboard_transitions_to_milestone_detail`: `App` with 1 milestone, selection `Some(0)`, screen `Dashboard`; `Enter`; assert `app.screen` is `MilestoneDetail`
   - `esc_returns_to_dashboard_from_milestone_detail`: `App` with screen `MilestoneDetail`; `Esc`; assert `app.screen` is `Dashboard`
   - `empty_milestones_does_not_change_list_state`: `App` with no milestones; `Down`; assert `list_state.selected() == None` (no panic)
   
   **Note on access**: Because T01 created `src/lib.rs` with all pub types, `tests/app_state.rs` can `use assay_tui::{App, Screen, handle_event}` directly — this is the standard Rust pattern for testable binary crates. The crate name is `assay-tui` → `assay_tui` in Rust identifiers.
   
   For milestone fixtures in tests: construct `Milestone` structs directly with `chrono::Utc::now()` for timestamps and empty optional fields. No disk I/O needed in these tests.
   
   Run `cargo test -p assay-tui` and fix any compilation or assertion failures. Then run `just ready` and fix fmt/lint issues.

## Must-Haves

- [ ] `KeyCode::Down` increments selection, wraps from last to first
- [ ] `KeyCode::Up` decrements selection, wraps from first to last
- [ ] Both ↑↓ are no-ops when `app.milestones.is_empty()`
- [ ] `KeyCode::Enter` on Dashboard with non-empty milestones sets `app.screen = Screen::MilestoneDetail`
- [ ] `KeyCode::Esc` sets `app.screen = Screen::Dashboard` from any other screen
- [ ] `draw_dashboard` guards empty list: renders placeholder `Paragraph`, does NOT call `render_stateful_widget` when milestones is empty
- [ ] `crates/assay-tui/tests/app_state.rs` exists with all 7 tests listed above
- [ ] `cargo test -p assay-tui` → all tests pass, 0 failed
- [ ] `just ready` → green

## Verification

- `cargo test -p assay-tui 2>&1 | grep 'test result'` → `test result: ok. 7 passed; 0 failed`
- `just ready` exits 0
- `grep -c 'is_empty' crates/assay-tui/src/main.rs` → ≥ 1 (confirms empty-state guard present)

## Observability Impact

- Signals added/changed: `handle_event` returning `false` on `q` is the sole quit signal; `App.screen` variant name is the readable state indicator for all navigation
- How a future agent inspects this: tests in `app_state.rs` serve as living documentation of every state transition — a future agent can read the tests to understand what each key does; `App.screen` is the single source of truth for current view
- Failure state exposed: test failures produce standard `cargo test` assertion output with expected vs actual values; no hidden state

## Inputs

- `crates/assay-tui/src/lib.rs` — T02 output with `draw_dashboard`, `draw_no_project` implemented; `handle_event` stub from T01
- `crates/assay-tui/src/main.rs` — T02 output with real data loading in `main()`
- `assay_types::milestone::Milestone` — for constructing test fixtures inline (no disk I/O)
- Research: `ListState` wrapping pattern, empty-list guard from S01-RESEARCH.md pitfalls section

## Expected Output

- `crates/assay-tui/src/lib.rs` — ↑↓/Enter/Esc navigation wired in `handle_event`; empty-list guard in `draw_dashboard`
- `crates/assay-tui/tests/app_state.rs` — 7 passing tests covering all state transitions
- `just ready` green
