---
estimated_steps: 6
estimated_files: 3
---

# T03: Refactor `run()` to channel-based event loop + wire `r` key handler

**Slice:** S01 — Channel Event Loop and Agent Run Panel
**Milestone:** M007

## Description

This task wires the runtime: replaces `main.rs::run()`'s blocking `event::read()` loop with a `mpsc::channel::<TuiEvent>()` receiver loop, spawns the crossterm event background thread, and implements the `r` key handler in the Dashboard arm of `handle_event()`. After this task, a developer pressing `r` on a project with an InProgress chunk will see `Screen::AgentRun` appear and lines stream in as the Claude agent runs.

The `r` key handler in `app.rs` is the most complex part:
1. Guard on `event_tx` being Some (tests leave it None → no-op)
2. Get active chunk slug via `cycle_status`
3. Build a minimal `HarnessProfile` and call the Claude harness adapter
4. Spawn the relay-wrapper thread: inner `launch_agent_streaming` + outer drain loop

The relay-wrapper thread pattern (inner thread + outer drain) ensures `AgentDone` is sent AFTER all `AgentLine` events, preventing line loss.

Key constraints per research doc:
- Use `mpsc::channel()` (unbounded) for both the `line_tx` inside `launch_agent_streaming` and the `TuiEvent` channel itself — bounded channels risk deadlock
- The crossterm background thread will block forever on `event::read()` after TUI exit — this is acceptable; the OS reclaims it on process exit (do NOT try to signal it)
- `write_config` for the harness needs a writable directory; for S01, write to a temp subdir of the assay_dir (e.g., `.assay/.claude-run-tmp/`) to avoid polluting the worktree root; clean approach: use `std::env::temp_dir().join("assay-agent-run")` and `std::fs::create_dir_all` before calling `write_config`

## Steps

1. **Refactor `main.rs::run()` to channel-based loop**
   - Add `use assay_tui::app::TuiEvent;` import to `main.rs`
   - Replace the function body with:
     ```rust
     fn run(mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
         let mut app = App::new()?;
         let (tx, rx) = std::sync::mpsc::channel::<TuiEvent>();
         app.event_tx = Some(tx.clone());

         // Background thread: crossterm events → TuiEvent channel
         std::thread::spawn(move || {
             loop {
                 match crossterm::event::read() {
                     Ok(crossterm::event::Event::Key(k)) => { let _ = tx.send(TuiEvent::Key(k)); }
                     Ok(crossterm::event::Event::Resize(w, h)) => { let _ = tx.send(TuiEvent::Resize(w, h)); }
                     _ => {}
                 }
             }
         });

         loop {
             terminal.draw(|frame| app.draw(frame))?;
             match rx.recv() {
                 Ok(TuiEvent::Key(k)) => { if app.handle_event(k) { break; } }
                 Ok(TuiEvent::Resize(..)) => { terminal.clear()?; }
                 Ok(TuiEvent::AgentLine(line)) => { app.handle_agent_line(line); }
                 Ok(TuiEvent::AgentDone { exit_code }) => { app.handle_agent_done(exit_code); }
                 Err(_) => break, // channel disconnected
             }
         }
         Ok(())
     }
     ```
   - Remove the old `use crossterm::event::{self, Event};` import (no longer needed in main.rs)

2. **Add `r` key handler to Dashboard arm in `app.rs`**
   - In the `Screen::Dashboard` match arm of `handle_event()`, add `KeyCode::Char('r')` case:
   - Guard: `if let Some(tx) = self.event_tx.clone()` — if None, `return false` (no-op in tests)
   - Get `assay_dir`: `let assay_dir = match &self.project_root { Some(root) => root.join(".assay"), None => return false, };`
   - Get active chunk: `let chunk_slug = match cycle_status(&assay_dir) { Ok(Some(cs)) => cs.active_chunk_slug.unwrap_or_else(|| cs.milestone_slug.clone()), _ => return false, };`
   - Build a minimal `HarnessProfile` for S01 (real provider dispatch in S02) — note `HarnessProfile` has no `Default` impl, construct explicitly: `let profile = assay_types::HarnessProfile { name: chunk_slug.clone(), prompt_layers: vec![], settings: assay_types::SettingsOverride { model: None, permissions: vec![], tools: vec![], max_turns: None }, hooks: vec![], working_dir: None };`
   - Generate harness config and write to temp dir:
     ```rust
     let run_dir = std::env::temp_dir().join(format!("assay-agent-{}", chunk_slug));
     let _ = std::fs::create_dir_all(&run_dir);
     let claude_config = assay_harness::claude::generate_config(&profile);
     if let Err(_) = assay_harness::claude::write_config(&claude_config, &run_dir) {
         return false;
     }
     let cli_args = assay_harness::claude::build_cli_args(&claude_config);
     let working_dir = run_dir.clone();
     ```
   - Transition to AgentRun screen: `self.screen = Screen::AgentRun { chunk_slug: chunk_slug.clone(), lines: vec![], scroll_offset: 0, status: AgentRunStatus::Running };`
   - Spawn relay-wrapper thread:
     ```rust
     let tui_tx = tx.clone();
     let handle = std::thread::spawn(move || {
         let (str_tx, str_rx) = std::sync::mpsc::channel::<String>();
         let inner = assay_core::pipeline::launch_agent_streaming(&cli_args, &working_dir, str_tx);
         // Drain lines → TuiEvent::AgentLine
         for line in str_rx {
             let _ = tui_tx.send(TuiEvent::AgentLine(line));
         }
         // All lines sent; get exit code
         let exit_code = inner.join().unwrap_or(-1);
         let _ = tui_tx.send(TuiEvent::AgentDone { exit_code });
         exit_code
     });
     self.agent_thread = Some(handle);
     ```

3. **Add `use assay_harness` and `use assay_core::pipeline` imports to `app.rs`**
   - Add `use assay_harness::claude as claude_harness;` (or inline the path)
   - Add `use assay_core::pipeline::launch_agent_streaming;`
   - Add `use assay_types::HarnessProfile;` if not already imported

4. **Verify existing tests still compile and pass**
   - Tests use `App::with_project_root(Some(root))` which sets `event_tx = None`
   - All `handle_event(key(...))` calls bypass `run()` entirely
   - The `r` key guard `if let Some(tx) = self.event_tx.clone()` returns false (no-op) when `event_tx` is None
   - Run `cargo test -p assay-tui` to confirm

5. **Add `use assay_types::SettingsOverride` import and confirm profile construction compiles**
   - `HarnessProfile` has no `Default` impl — use explicit struct construction (see Step 2)
   - This is S01 MVP — S02 replaces this with real provider dispatch from `app.config`

6. **Smoke-test the refactored `run()` with integration tests**
   - `cargo test -p assay-tui` — all tests must pass
   - `cargo build -p assay-tui` — binary produced
   - Verify the r-key agent_run test passes: `cargo test -p assay-tui --test agent_run -- r_key_noops_when_event_tx_is_none`

## Must-Haves

- [ ] `run()` uses `mpsc::Receiver<TuiEvent>` instead of `event::read()`
- [ ] Background crossterm thread spawned; sends `TuiEvent::Key` and `TuiEvent::Resize`
- [ ] `app.event_tx = Some(tx.clone())` set before the main loop
- [ ] Main loop dispatches `AgentLine` → `handle_agent_line`, `AgentDone` → `handle_agent_done`
- [ ] `r` key in Dashboard: no-op when `event_tx` is None; transitions to `Screen::AgentRun` and spawns relay-wrapper thread when wired
- [ ] Relay-wrapper thread: drains String lines → `TuiEvent::AgentLine`; then joins inner JoinHandle; then sends `TuiEvent::AgentDone { exit_code }`
- [ ] `self.agent_thread` stores the wrapper's `JoinHandle<i32>`
- [ ] All 27 pre-existing TUI tests still pass
- [ ] All 8 agent_run tests still pass
- [ ] `cargo build -p assay-tui` succeeds

## Verification

```bash
# Smoke test: binary builds
cargo build -p assay-tui

# All TUI tests (including new agent_run tests)
cargo test -p assay-tui

# Specific r-key no-op test
cargo test -p assay-tui --test agent_run -- r_key_noops_when_event_tx_is_none

# No clippy errors
cargo clippy -p assay-tui
```

## Observability Impact

- Signals added/changed: `run()` now dispatches `TuiEvent::AgentLine` and `TuiEvent::AgentDone` — these are the runtime signals that drive the agent streaming UI.
- How a future agent inspects this: `app.agent_thread.is_some()` indicates an active streaming agent; `app.screen` shows `Screen::AgentRun { status: AgentRunStatus::Running, .. }` during execution.
- Failure state exposed: If `write_config` fails in the `r` handler, the transition is silently skipped (screen stays on Dashboard). S02 can add a visible error message to the Dashboard. If the relay-wrapper thread panics, `rx.recv()` returns `Err(_)` and the TUI exits gracefully (the `Err(_) => break` arm in `run()`).

## Inputs

- `crates/assay-tui/src/app.rs` (T02 output) — real `handle_agent_line`, `handle_agent_done`, `Screen::AgentRun` variant with working draw/event arms
- `crates/assay-core/src/pipeline.rs` (T01 output) — `launch_agent_streaming` implemented
- `crates/assay-harness/src/claude.rs` — `generate_config`, `write_config`, `build_cli_args` (existing)
- `crates/assay-core/src/milestone/cycle.rs` — `cycle_status` (existing)
- D107, D108 (channel-based event loop decisions)
- Research doc "Common Pitfalls" — crossterm thread never exits (acceptable), relay-wrapper ordering guarantee

## Expected Output

- `crates/assay-tui/src/main.rs` — channel-based `run()` with background crossterm thread
- `crates/assay-tui/src/app.rs` — `r` key handler in Dashboard arm; `use assay_harness::claude` import
- `target/debug/assay-tui` — binary produced by `cargo build -p assay-tui`
- All 35 TUI tests (27 pre-existing + 8 agent_run) green
