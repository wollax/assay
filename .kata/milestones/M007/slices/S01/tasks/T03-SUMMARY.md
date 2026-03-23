---
id: T03
parent: S01
milestone: M007
provides:
  - "Real `handle_tui_event` dispatching AgentLine into Screen::AgentRun.lines and AgentDone into AgentStatus::Done/Failed"
  - "`r` key on Dashboard spawns agent via launch_agent_streaming and transitions to Screen::AgentRun"
  - "Esc key on AgentRun returns to Dashboard"
  - "Real `draw_agent_run` with scrollable list, status line, and hint bar"
  - "`App.event_tx: Option<mpsc::Sender<TuiEvent>>` for cross-thread event delivery"
  - "Channel-based run() loop in main.rs replacing blocking event::read()"
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/src/main.rs
key_decisions:
  - "Forwarder thread pattern: a wrapper thread receives from line_rx and sends AgentLine/AgentDone through event_tx; the JoinHandle<i32> from launch_agent_streaming stays in App.agent_thread and is joined in handle_tui_event on AgentDone"
  - "S01 r-key sends AgentDone { exit_code: 0 } as sentinel from forwarder thread; real exit code from JoinHandle is consumed in handle_tui_event.take() — full exit-code accuracy is S02's concern"
  - "event_tx is Option<Sender<TuiEvent>> on App (initialized None, set by run() before loop); guards in the r handler return false when None (safe for tests that don't call run())"
patterns_established:
  - "mpsc::channel::<TuiEvent>() with background crossterm reader thread; main loop drives terminal.draw + event dispatch via rx.recv()"
  - "Agent forwarder thread relays line_rx lines as AgentLine events and sends AgentDone when channel disconnects"
  - "draw_agent_run: Layout::vertical [1, Fill, 1, 1] for title/list/status/hint; render_stateful_widget for the output list"
observability_surfaces:
  - "Screen::AgentRun { lines, status, .. } — inspect app.screen after driving handle_tui_event events"
  - "AgentStatus::Running / Done { exit_code } / Failed { exit_code } — visible in status line and from test assertions"
  - "draw_agent_run renders 'Running…', 'Done (exit 0)', or 'Failed (exit N)'"
duration: ~1h
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T03: Implement `handle_tui_event`, `r` key, `draw_agent_run`, and refactor `run()`

**Replaced the `handle_tui_event` stub with real AgentLine/AgentDone dispatch, wired the `r` key to spawn the agent, implemented `draw_agent_run` with scrollable list, added Esc→Dashboard on AgentRun, and replaced the blocking `event::read()` main loop with an `mpsc::Receiver<TuiEvent>` channel loop — all 31 tests pass.**

## What Happened

1. Added `use std::sync::mpsc` import and `event_tx: Option<mpsc::Sender<TuiEvent>>` field to `App` (initialized `None` in `with_project_root`).

2. Implemented `handle_tui_event`:
   - `AgentLine(s)`: if in `Screen::AgentRun`, pushes `s` to `lines`.
   - `AgentDone { exit_code }`: sets `status` to `Done { exit_code: 0 }` or `Failed { exit_code }`, calls `self.agent_thread.take().map(|h| let _ = h.join())`, then reloads milestones and `cycle_slug` from disk.
   - `Key`/`Resize`: no-op (handled by `run()` loop).

3. Added `r` key handler in `Screen::Dashboard` arm:
   - Guards: `project_root.is_some()`, active chunk exists via `cycle_status`, `event_tx.is_some()`.
   - Builds minimal `cli_args = ["claude", "--print"]` for S01.
   - Creates `mpsc::channel::<String>()` for line delivery.
   - Calls `launch_agent_streaming(&cli_args, &working_dir, line_tx)` and stores the handle in `self.agent_thread`.
   - Spawns a forwarder thread that reads `line_rx` and sends `TuiEvent::AgentLine` per line, then sends `TuiEvent::AgentDone { exit_code: 0 }` on channel disconnect.
   - Transitions to `Screen::AgentRun { chunk_slug, lines: vec![], scroll_offset: 0, status: AgentStatus::Running }`.

4. Added Esc handler to `Screen::AgentRun` arm: transitions to `Screen::Dashboard`.

5. Implemented `draw_agent_run` with `Layout::vertical([Length(1), Fill(1), Length(1), Length(1)])` producing title/list/status/hint areas; `render_stateful_widget` for the output list.

6. Rewrote `main.rs::run()`: creates `(tx, rx): mpsc::channel::<TuiEvent>()`, sets `app.event_tx = Some(tx.clone())`, spawns background crossterm reader thread, replaces main loop with `while let Ok(event) = rx.recv()` dispatching Key/Resize/AgentLine/AgentDone.

## Verification

```
cargo test -p assay-tui
```
Output: 31 tests passed (4 new agent_run + 27 existing), 0 failed.

New tests passing:
- `agent_line_events_accumulate_in_agent_run_screen`
- `agent_done_event_transitions_to_done_status`
- `agent_done_nonzero_exit_sets_failed_status`
- `r_key_no_active_chunk_is_noop`

`cargo build -p assay-tui` — clean, no warnings.

`grep -n "event::read()" crates/assay-tui/src/main.rs` — only appears inside the background thread, not in the main loop.

## Diagnostics

- `AgentStatus` transitions: `matches!(app.screen, Screen::AgentRun { status: AgentStatus::Done { .. }, .. })` after driving `handle_tui_event(AgentDone { exit_code: 0 })`.
- Accumulated lines: `if let Screen::AgentRun { lines, .. } = &app.screen { assert_eq!(lines, ...) }`.
- Status rendered in TUI as "Running…", "Done (exit 0)", or "Failed (exit N)".

## Deviations

- **Exit code from forwarder thread**: The forwarder thread always sends `AgentDone { exit_code: 0 }` as a sentinel when the line channel disconnects. The real exit code from `launch_agent_streaming`'s `JoinHandle<i32>` is discarded when `handle_tui_event` calls `agent_thread.take().map(|h| let _ = h.join())`. This matches the S01 plan's note ("UAT proves the real args work") — full exit-code accuracy is deferred to S02 where the real CLI args and harness dispatch are wired.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — `handle_tui_event` implemented; `r` key wired; `draw_agent_run` real implementation; `App.event_tx` field added; `Screen::AgentRun` Esc handler added
- `crates/assay-tui/src/main.rs` — channel-based `run()` loop; blocking `event::read()` loop replaced with `mpsc::Receiver<TuiEvent>` dispatch
