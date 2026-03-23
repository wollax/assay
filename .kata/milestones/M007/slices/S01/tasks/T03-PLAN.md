---
estimated_steps: 6
estimated_files: 2
---

# T03: Refactor `run()` to channel-based `TuiEvent` loop and implement `AgentRun` rendering

**Slice:** S01 — Channel Event Loop and Agent Run Panel
**Milestone:** M007

## Description

Replace the blocking `event::read()` loop in `main.rs` with a channel-based `TuiEvent` dispatch loop. Implement `handle_agent_line`, `handle_agent_done`, and `draw_agent_run` in `app.rs`. This task does NOT wire the `r` key to actual agent spawning — that is T04. After this task, the TUI compiles and all 27 existing tests still pass.

The crossterm background thread uses `event::read()` (blocking) in a loop and sends `TuiEvent::Key`/`TuiEvent::Resize` to a `Sender<TuiEvent>`. The main loop becomes `while let Ok(event) = rx.recv()`. Agent-related `TuiEvent` variants are handled but the `r` key dispatch to actual spawning is still a no-op at this stage.

## Steps

1. In `crates/assay-tui/src/main.rs`, define the `TuiEvent` enum:
   ```rust
   pub enum TuiEvent {
       Key(crossterm::event::KeyEvent),
       Resize(u16, u16),
       AgentLine(String),
       AgentDone { exit_code: i32 },
   }
   ```
   Make it `pub` so `app.rs` and tests can reference it.

2. Refactor `run()` in `main.rs`:
   - Create `mpsc::channel::<TuiEvent>()`
   - Spawn crossterm background thread: `let tx_cross = tx.clone(); std::thread::spawn(move || loop { if let Ok(e) = event::read() { match e { Event::Key(k) => { let _ = tx_cross.send(TuiEvent::Key(k)); } Event::Resize(w, h) => { let _ = tx_cross.send(TuiEvent::Resize(w, h)); } _ => {} } } });`
   - Main loop: `loop { terminal.draw(|frame| app.draw(frame))?; match rx.recv() { Ok(TuiEvent::Key(key)) => { if app.handle_event(key) { break; } } Ok(TuiEvent::Resize(..)) => { terminal.clear()?; } Ok(TuiEvent::AgentLine(line)) => { app.handle_agent_line(line); } Ok(TuiEvent::AgentDone { exit_code }) => { app.handle_agent_done(exit_code); } Err(_) => break, } }`
   - Remove `use crossterm::event::{self, Event}` if now unused in main; add `use std::sync::mpsc`

3. In `crates/assay-tui/src/app.rs`, implement `handle_agent_line`:
   ```rust
   pub fn handle_agent_line(&mut self, line: String) {
       if let Screen::AgentRun { ref mut lines, .. } = self.screen {
           lines.push(line);
           // scroll_offset updated in draw_agent_run dynamically
       }
   }
   ```

4. In `app.rs`, implement `handle_agent_done`:
   ```rust
   pub fn handle_agent_done(&mut self, exit_code: i32) {
       // Join the background thread for cleanup
       if let Some(handle) = self.agent_thread.take() {
           let _ = handle.join();
       }
       // Refresh milestone data
       if let Some(ref root) = self.project_root {
           let assay_dir = root.join(".assay");
           self.milestones = milestone_scan(&assay_dir).unwrap_or_default();
           self.cycle_slug = cycle_status(&assay_dir).ok().flatten()
               .map(|s| s.milestone_slug);
       }
       // Update AgentRun status
       let new_status = if exit_code == 0 {
           AgentRunStatus::Done { exit_code }
       } else {
           AgentRunStatus::Failed { exit_code }
       };
       if let Screen::AgentRun { ref mut status, .. } = self.screen {
           *status = new_status;
       }
   }
   ```
   Honor D098: scan first (borrows `self.project_root`), mutate `self.screen` after.

5. In `app.rs`, implement `draw_agent_run(frame, area, chunk_slug, lines, scroll_offset, status)`:
   - Outer block with title "Agent Run: {chunk_slug}"
   - Inner layout: content area (most of height) + status line (1 row)
   - Content area: `List` widget showing lines; auto-scroll: compute visible height, pass `scroll_offset = lines.len().saturating_sub(visible_height)` — store it back into the screen variant before render (or compute and pass as parameter). For simplicity, compute visible height from `area.height.saturating_sub(3)` (borders + status line).
   - Status line: match `status` → "Running..." (yellow) / "Done (exit 0)" (green) / "Failed (exit N)" (red)
   - Honor D097: accepts explicit fields, not `&mut App`; honor D105: receives `area` from `App::draw()`

6. Add `Screen::AgentRun` arm to `draw()` in `App`:
   ```rust
   Screen::AgentRun { ref chunk_slug, ref lines, scroll_offset, ref status } => {
       draw_agent_run(frame, content_area, chunk_slug, lines, *scroll_offset, status);
   }
   ```
   Note: `scroll_offset` may need to be computed and stored; keep it simple — update `scroll_offset` in `handle_agent_line` as `lines.len().saturating_sub(visible_height)` where `visible_height` is a reasonable constant (e.g., 20) or passed in. Exact auto-scroll precision is secondary to correctness.

## Must-Haves

- [ ] `TuiEvent` enum is `pub` in `main.rs` with Key, Resize, AgentLine, AgentDone variants
- [ ] `run()` uses `mpsc::Receiver<TuiEvent>`; crossterm thread sends Key/Resize; no blocking `event::read()` in main loop
- [ ] `handle_agent_line` appends to `Screen::AgentRun.lines` when screen is AgentRun
- [ ] `handle_agent_done` joins `agent_thread`, refreshes milestones + cycle_slug, updates `Screen::AgentRun.status`
- [ ] `draw_agent_run` renders: scrollable lines list + status line at bottom; accepts individual field params (D097)
- [ ] `draw()` has `Screen::AgentRun` arm calling `draw_agent_run`
- [ ] All 27 existing `cargo test -p assay-tui` tests still pass
- [ ] `cargo build -p assay-tui` zero warnings

## Verification

- `cargo test -p assay-tui 2>&1 | tail -3` — "test result: ok. 27 passed"
- `cargo build -p assay-tui 2>&1 | grep "^error"` — empty
- `cargo test -p assay-tui --test agent_run -- agent_run_streams_lines_and_transitions_to_done` — should pass (T01 test now exercisable via handle_agent_line/handle_agent_done)
- `cargo test -p assay-tui --test agent_run -- agent_run_failed_exit_code_shows_failed_status` — should pass

## Observability Impact

- Signals added/changed: `TuiEvent` enum is the new event bus; `Screen::AgentRun.lines` accumulates all agent stdout for post-mortem inspection; `Screen::AgentRun.status` transitions: Running → Done/Failed
- How a future agent inspects this: `match &app.screen { Screen::AgentRun { lines, status, .. } => { /* inspect */ } }`
- Failure state exposed: `AgentRunStatus::Failed { exit_code }` is visually rendered in red in the status line; `handle_agent_done` refreshes milestones so gate results update immediately

## Inputs

- `crates/assay-tui/src/main.rs` — current 30-line blocking loop to be replaced
- `crates/assay-tui/src/app.rs` — stub scaffolding from T01 (AgentRunStatus, Screen::AgentRun, method stubs) to be filled in
- `crates/assay-tui/tests/agent_run.rs` — T01 tests that `handle_agent_line`/`handle_agent_done` must now make pass

## Expected Output

- `crates/assay-tui/src/main.rs` — channel-based `TuiEvent` loop; background crossterm thread
- `crates/assay-tui/src/app.rs` — `handle_agent_line`, `handle_agent_done`, `draw_agent_run` implemented; `draw()` updated
- `agent_run_streams_lines_and_transitions_to_done` and `agent_run_failed_exit_code_shows_failed_status` tests green
- All 27 existing TUI tests still pass
