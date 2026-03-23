---
estimated_steps: 6
estimated_files: 4
---

# T02: Add `TuiEvent`, `AgentStatus`, `Screen::AgentRun`, and `App` fields

**Slice:** S01 — Channel Event Loop and Agent Run Panel
**Milestone:** M007

## Description

Establish the data model for agent execution in the TUI. This task adds all new types (`TuiEvent`, `AgentStatus`, `Screen::AgentRun`) and new `App` fields (`agent_thread`, `agent_list_state`) to `app.rs`, initializes them in `with_project_root`, adds a stub `draw_agent_run` function that satisfies compilation but panics if called, adds a stub `handle_tui_event` method, and creates `crates/assay-tui/tests/agent_run.rs` with all four integration tests written in failing/red state.

The key constraint: **all 27 existing TUI tests must continue to pass** after this task. Adding fields to `App` requires updating `with_project_root`. Adding a `Screen::AgentRun` variant requires updating all match exhaustion sites (primarily `draw()` and `handle_event()`).

Tests are written now (even though some will fail until T03 completes the implementation) so they drive and constrain the T03 implementation precisely.

## Steps

1. In `crates/assay-tui/src/app.rs`, add `pub enum TuiEvent` with variants `Key(crossterm::event::KeyEvent)`, `Resize(u16, u16)`, `AgentLine(String)`, `AgentDone { exit_code: i32 }`. Place it near the top of the file after the imports. This type is public so integration tests can construct events.

2. Add `pub enum AgentStatus` with variants `Running`, `Done { exit_code: i32 }`, `Failed { exit_code: i32 }`. Place it adjacent to `TuiEvent`.

3. Add `Screen::AgentRun { chunk_slug: String, lines: Vec<String>, scroll_offset: usize, status: AgentStatus }` to the `Screen` enum. Update all `match &self.screen` and `match self.screen` exhaustion sites in `draw()` and `handle_event()`: add an arm for `Screen::AgentRun { .. }` in `draw()` that calls `draw_agent_run` (stub); add an arm in `handle_event()` for `Screen::AgentRun { .. }` that returns `false` (Esc to Dashboard will be added in T03).

4. Add two new fields to `App`: `pub agent_thread: Option<std::thread::JoinHandle<i32>>` and `pub agent_list_state: ListState`. Initialize both in `with_project_root`: `agent_thread: None, agent_list_state: ListState::default()`.

5. Add stub method to `App`: `pub fn handle_tui_event(&mut self, event: TuiEvent) -> bool { false }`. This will be replaced in T03 with the real implementation. The stub allows `agent_run.rs` tests to compile.

6. Add stub `draw_agent_run` free function at the bottom of `app.rs` (before the final `find_project_root` function): `fn draw_agent_run(frame: &mut ratatui::Frame, area: Rect, chunk_slug: &str, lines: &[String], _scroll_offset: usize, status: &AgentStatus, _list_state: &mut ListState) { /* T03 implements this */ }`. Wire it in the `Screen::AgentRun` draw arm using `..` pattern and placeholder args.

7. Create `crates/assay-tui/tests/agent_run.rs` with the following tests (see Verification). Import `assay_tui::app::{App, Screen, TuiEvent, AgentStatus}`. Some tests will fail until T03 — that is expected and correct.

## Must-Haves

- [ ] `TuiEvent`, `AgentStatus`, `Screen::AgentRun` all compile without error
- [ ] `App.agent_thread: Option<JoinHandle<i32>>` and `App.agent_list_state: ListState` fields initialized in `with_project_root`
- [ ] `App::handle_tui_event` stub method exists and compiles
- [ ] `draw_agent_run` stub exists and is wired in `draw()`
- [ ] `crates/assay-tui/tests/agent_run.rs` exists with all 4 tests from Verification
- [ ] All 27 existing TUI tests pass (zero regressions)
- [ ] `cargo build -p assay-tui` succeeds with no warnings

## Verification

- `cargo test -p assay-tui` — all 27 existing tests green; `agent_run.rs` tests compile (some may fail — correct)
- `cargo build -p assay-tui` — clean
- Test file `crates/assay-tui/tests/agent_run.rs` must contain:
  ```
  fn agent_line_events_accumulate_in_agent_run_screen
  fn agent_done_event_transitions_to_done_status
  fn agent_done_nonzero_exit_sets_failed_status
  fn r_key_no_active_chunk_is_noop
  ```
- `grep -n "agent_thread\|agent_list_state" crates/assay-tui/src/app.rs` — both fields visible

## Observability Impact

- Signals added/changed: `AgentStatus` enum is the primary runtime signal for agent execution state — `Running`, `Done`, `Failed` with exit code. These are observable from `app.screen` in tests and rendered in T03.
- How a future agent inspects this: construct `App::with_project_root(Some(root))`, drive `handle_tui_event`, assert on `app.screen` variant and fields — same pattern as all existing TUI integration tests.
- Failure state exposed: `AgentStatus::Failed { exit_code }` and `AgentStatus::Done { exit_code }` both carry the exit code — observable from screen state.

## Inputs

- `crates/assay-tui/src/app.rs` — existing `Screen` enum, `App` struct, `draw()`, `handle_event()`, `with_project_root()` are all modified
- T01 output — `launch_agent_streaming` is pub in assay-core (imported in T03, not T02)
- `crates/assay-tui/tests/settings.rs` — reference for integration test pattern: `App::with_project_root(Some(root)).unwrap()`, `handle_event(key(KeyCode::...))`, `assert!(matches!(...)))`
- D107 — `TuiEvent` enum design; D097 — individual fields in render fns; D098 — `..` pattern in match arms

## Expected Output

- `crates/assay-tui/src/app.rs` — extended with new types, fields, stub method, stub render fn
- `crates/assay-tui/tests/agent_run.rs` — new test file with 4 tests
- `cargo test -p assay-tui` green for all 27 existing tests; new tests compile
