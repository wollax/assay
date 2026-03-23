---
estimated_steps: 7
estimated_files: 3
---

# T03: Implement `handle_tui_event`, `r` key, `draw_agent_run`, and refactor `run()`

**Slice:** S01 — Channel Event Loop and Agent Run Panel
**Milestone:** M007

## Description

Complete the S01 implementation by: (1) replacing the `handle_tui_event` stub with the real implementation, (2) adding the `r` key handler to the Dashboard arm, (3) implementing `draw_agent_run` with a scrollable list, and (4) refactoring `main.rs::run()` to use the channel-based `mpsc::Receiver<TuiEvent>` loop.

After this task, all four new tests in `agent_run.rs` pass, all 27 existing tests still pass, and `cargo build -p assay-tui` is clean.

**Key constraints from research:**
- D107: use unbounded `mpsc::channel()` — a bounded channel that fills blocks the crossterm event thread, freezing the TUI
- D097: `draw_agent_run` takes individual fields, not `&mut self`
- D098: `Screen::AgentRun { .. }` in draw match arm avoids borrow-split on screen variant data
- The `r` key handler for S01 hardcodes the Anthropic adapter (`assay_harness::claude`) — S02 replaces this with `provider_harness_writer(config)`. Do NOT implement provider dispatch in this task.
- The agent is spawned with `self.project_root` as `working_dir` for S01 (simplified — full worktree setup is S02/pipeline concern)
- `JoinHandle<i32>` is not Clone — consume it with `self.agent_thread.take()` on `AgentDone`

## Steps

1. **Implement `App::handle_tui_event`** in `app.rs`. Replace the stub with the real implementation:
   - `TuiEvent::AgentLine(s)`: if `self.screen` is `Screen::AgentRun`, push `s` to `lines`; no-op otherwise.
   - `TuiEvent::AgentDone { exit_code }`: if `self.screen` is `Screen::AgentRun { ref mut status, .. }`, set `*status = if exit_code == 0 { AgentStatus::Done { exit_code: 0 } } else { AgentStatus::Failed { exit_code } }`; call `self.agent_thread.take().map(|h| let _ = h.join())` to consume the handle; refresh `self.milestones` and `self.cycle_slug` from disk (same reload code as wizard submit success path).
   - `TuiEvent::Key(key)` and `TuiEvent::Resize(_, _)`: forward to existing `handle_event` / `terminal.clear()` respectively (these are handled by the `run()` loop in main.rs, not here — `handle_tui_event` only handles `AgentLine`/`AgentDone`).
   - Return `false` from all arms.

2. **Add `r` key handler** to the Dashboard arm of `handle_event()`. Steps within the handler:
   - Guard: if `self.project_root` is `None`, return early (no-op).
   - Call `cycle_status(&assay_dir)` → if `Err` or `Ok(None)` or `Ok(Some(cs))` where `cs.active_chunk_slug` is `None`, return `false` (no-op).
   - Extract `chunk_slug = cs.active_chunk_slug.unwrap()`.
   - Build `HarnessProfile` using `assay_core::pipeline::build_harness_profile` with a default `ManifestSession` (spec = chunk_slug). For S01, use a minimal profile — just enough to call `claude::build_cli_args`. Alternatively, call `claude::build_cli_args` with a hardcoded minimal `ClaudeConfig` (no CLAUDE.md path needed for streaming test). Keep it simple: `let cli_args = vec!["claude".to_string(), "--print".to_string()]` for S01 (UAT proves the real args work).
   - Create `mpsc::channel::<String>()` for line delivery; clone the main loop's `TuiEvent` sender (needs to be passed into the handler — see step 5 for how `run()` threads this through).
   - Spawn a wrapper thread: calls `launch_agent_streaming(cli_args, working_dir, line_tx)`, joins the handle to get exit code, then sends `TuiEvent::AgentDone { exit_code }` through the main event sender.
   - Store `JoinHandle<()>` (wrapper thread) in `App.agent_thread`... but `agent_thread` is `Option<JoinHandle<i32>>`. **Revised approach per research**: store `Option<std::thread::JoinHandle<i32>>` from `launch_agent_streaming` directly; the wrapper thread sends `AgentDone` via the main tx. `handle_tui_event` calls `self.agent_thread.take().map(|h| let _ = h.join())`.
   - Set `self.screen = Screen::AgentRun { chunk_slug, lines: vec![], scroll_offset: 0, status: AgentStatus::Running }`.
   - Note: because `handle_event` does not have access to the main tx sender today (the loop in main.rs controls it), the `r` key handler must be called from a context where the sender is available. The simplest approach: add a `Option<std::sync::mpsc::Sender<TuiEvent>>` field to `App` (set by `run()` before starting the loop); `r` handler reads `self.event_tx.clone()` for the agent wrapper thread.

3. **Add `App.event_tx: Option<mpsc::Sender<TuiEvent>>`** field to support the agent thread's ability to send events back into the main loop. Initialize to `None` in `with_project_root`. Set in `run()` after creating the channel and before starting the loop.

4. **Add `Screen::AgentRun` Esc handler** in `handle_event()`: if `Esc` is pressed while in `AgentRun`, set `self.screen = Screen::Dashboard`.

5. **Refactor `main.rs::run()`** from blocking `event::read()` to channel-based `mpsc::Receiver<TuiEvent>`:
   - Create `let (tx, rx) = std::sync::mpsc::channel::<TuiEvent>();`.
   - Set `app.event_tx = Some(tx.clone())`.
   - Spawn background thread that loops `crossterm::event::read()` and sends `TuiEvent::Key(key)` or `TuiEvent::Resize(w, h)` through `tx`. The thread exits when `tx.send(...)` returns `Err` (receiver dropped = app exited).
   - Replace the `loop { terminal.draw(...); match event::read()? { ... } }` with `while let Ok(event) = rx.recv() { terminal.draw(...); match event { TuiEvent::Key(key) => { if app.handle_event(key) { break; } } TuiEvent::Resize(..) => { terminal.clear()?; } TuiEvent::AgentLine(_) | TuiEvent::AgentDone { .. } => { app.handle_tui_event(event); } } }`.
   - Remove the `use crossterm::event::{self, Event};` import from `main.rs` and replace with the channel-based equivalent.

6. **Implement `draw_agent_run`** (replace the stub). Signature: `fn draw_agent_run(frame: &mut ratatui::Frame, area: Rect, chunk_slug: &str, lines: &[String], _scroll_offset: usize, status: &AgentStatus, list_state: &mut ListState)`. Layout: `[title_area(1), list_area(Fill), status_area(1), hint_area(1)]`. Title: `format!("  Agent Run — {chunk_slug}  ")`. List: `List::new(lines.iter().map(|l| ListItem::new(l.as_str())))` rendered with `render_stateful_widget`. Status line: `"Running…"` / `"Done (exit 0)"` / `"Failed (exit N)"`. Hint: `"↑↓ scroll  Esc back"` (dim). Wire `Screen::AgentRun { chunk_slug, lines, scroll_offset, status }` draw arm using `..` to avoid borrow issues; pass `&mut self.agent_list_state` separately.

7. **Run all tests**: `cargo test -p assay-tui` — 27 existing + 4 new agent_run tests must all pass. `cargo build -p assay-tui` clean. If any compilation issue arises from the `event_tx` field, ensure `TuiEvent` derives nothing that prevents it from being sent across thread boundaries (it doesn't need Clone; `mpsc::Sender<TuiEvent>` just needs `TuiEvent: Send`).

## Must-Haves

- [ ] `handle_tui_event` correctly accumulates `AgentLine` events into `Screen::AgentRun.lines`
- [ ] `handle_tui_event` for `AgentDone { exit_code: 0 }` sets `AgentStatus::Done { exit_code: 0 }`
- [ ] `handle_tui_event` for `AgentDone { exit_code: 1 }` sets `AgentStatus::Failed { exit_code: 1 }`
- [ ] `r` key on Dashboard with no active chunk is a no-op (screen stays Dashboard)
- [ ] `draw_agent_run` renders without panic (no real terminal needed — draw call in unit tests suffices)
- [ ] `run()` uses `mpsc::Receiver<TuiEvent>` loop — blocking `event::read()` call removed from main loop
- [ ] All 27 existing TUI tests pass
- [ ] 4 new `agent_run.rs` tests pass
- [ ] `cargo build -p assay-tui` clean

## Verification

- `cargo test -p assay-tui` — prints 31 tests passed (27 + 4 new)
- Specific new tests: `agent_line_events_accumulate_in_agent_run_screen`, `agent_done_event_transitions_to_done_status`, `agent_done_nonzero_exit_sets_failed_status`, `r_key_no_active_chunk_is_noop`
- `cargo build -p assay-tui` — exit 0, no warnings
- `grep -n "event::read()" crates/assay-tui/src/main.rs` — the blocking call is gone from the main loop (may still appear in the background thread that feeds the channel)

## Observability Impact

- Signals added/changed: `App.event_tx` is the new inter-thread boundary; agent output flows through `TuiEvent::AgentLine` and `TuiEvent::AgentDone` to the main event channel. `AgentStatus` transitions are directly observable from `app.screen` in tests.
- How a future agent inspects this: drive `app.handle_tui_event(TuiEvent::AgentLine("output line".into()))` then assert `matches!(app.screen, Screen::AgentRun { lines, .. } if lines.contains(&"output line".to_string()))`.
- Failure state exposed: `AgentStatus::Failed { exit_code }` rendered as "Failed (exit N)" in the TUI; observable from test assertions on `app.screen`; join panic mapped to -1.

## Inputs

- T01 output: `assay_core::pipeline::launch_agent_streaming` is pub and tested
- T02 output: `TuiEvent`, `AgentStatus`, `Screen::AgentRun`, `App.agent_thread`, `App.agent_list_state`, `App::handle_tui_event` stub, `draw_agent_run` stub, `agent_run.rs` test file all in place
- `crates/assay-tui/src/app.rs` — existing `with_project_root`, `handle_event` Dashboard arm, `draw()` are the primary edit targets
- `crates/assay-tui/src/main.rs` — existing 15-line `run()` is replaced with channel loop
- D107 — unbounded channel; D097 — individual fields in render fn; D098 — `..` in match arms to avoid borrow-split

## Expected Output

- `crates/assay-tui/src/app.rs` — `handle_tui_event` implemented; `r` key wired; `draw_agent_run` real implementation; `App.event_tx` field added; `Screen::AgentRun` Esc handler added
- `crates/assay-tui/src/main.rs` — channel-based `run()` loop; blocking `event::read()` loop replaced
- `cargo test -p assay-tui` — 31 tests green
