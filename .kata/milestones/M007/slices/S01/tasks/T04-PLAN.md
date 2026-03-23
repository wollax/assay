---
estimated_steps: 5
estimated_files: 4
---

# T04: Wire `r` key handler, add `assay-harness` dependency, complete integration tests

**Slice:** S01 — Channel Event Loop and Agent Run Panel
**Milestone:** M007

## Description

Complete the slice by: (1) adding `assay-harness` to `assay-tui` dependencies; (2) implementing the `r` key handler in `App::handle_event` Dashboard arm to spawn the agent via `launch_agent_streaming` and transition to `Screen::AgentRun`; (3) making the final integration test (`agent_run_r_key_on_no_project_is_noop`) pass; (4) running `just ready` to confirm the milestone is green.

The `r` handler needs access to `event_tx: mpsc::Sender<TuiEvent>` to wire the agent-output thread to the event loop. The cleanest approach is to store the sender on `App` (as `App.event_tx: Option<mpsc::Sender<TuiEvent>>`) and set it from `run()` after constructing the channel. This avoids threading the sender through every `handle_event` call.

## Steps

1. Add `assay-harness.workspace = true` to `[dependencies]` in `crates/assay-tui/Cargo.toml`.

2. Add `App.event_tx: Option<mpsc::Sender<TuiEvent>>` field to `App` (initialize to `None` in `with_project_root`). In `run()`, after creating the channel: `app.event_tx = Some(tx.clone());`. Add `use std::sync::mpsc` to `app.rs`.

3. Implement the `r` key handler in `App::handle_event` Dashboard arm:
   ```rust
   KeyCode::Char('r') => {
       if let Some(ref root) = self.project_root {
           let assay_dir = root.join(".assay");
           if let Ok(Some(status)) = cycle_status(&assay_dir) {
               if let Some(ref chunk_slug) = status.active_chunk_slug {
                   if let Some(ref event_tx) = self.event_tx {
                       // Build a minimal HarnessProfile for the active chunk
                       let session = assay_types::ManifestSession {
                           spec: chunk_slug.clone(),
                           ..Default::default()
                       };
                       let profile = assay_core::pipeline::build_harness_profile(&session);
                       let claude_config = assay_harness::claude::generate_config(&profile);
                       // Write harness config to a temp dir
                       let tmp = tempfile::tempdir().expect("tempdir");
                       let _ = assay_harness::claude::write_config(&claude_config, tmp.path());
                       let cli_args = assay_harness::claude::build_cli_args(&claude_config);
                       
                       let (line_tx, line_rx) = std::sync::mpsc::channel::<String>();
                       let working_dir = root.clone();
                       let handle = assay_core::pipeline::launch_agent_streaming(
                           &cli_args, &working_dir, line_tx,
                       );
                       self.agent_thread = Some(handle);
                       
                       // Bridge line_rx → TuiEvent channel
                       let event_tx2 = event_tx.clone();
                       std::thread::spawn(move || {
                           // recv lines and forward as AgentLine; on disconnect send AgentDone
                           // We can't get exit code here directly (handle was moved to App)
                           // Instead: drain line_rx, then send AgentDone with 0 as sentinel
                           // The real exit code is sent via a second channel approach.
                           // REVISED APPROACH: use a separate exit-code channel.
                           // See implementation note below.
                           for line in line_rx {
                               let _ = event_tx2.send(TuiEvent::AgentLine(line));
                           }
                           // line_rx disconnected = agent stdout closed = agent done
                           // But we don't have the exit code here. Use -999 as sentinel;
                           // handle_agent_done will join the actual thread via agent_thread.
                           // Actually, we need a different design — see note.
                           let _ = event_tx2.send(TuiEvent::AgentDone { exit_code: 0 });
                       });
                       // Keep tmp alive for the duration of the run
                       // (store it on App or leak it — leak is simplest for M007)
                       std::mem::forget(tmp);
                       
                       self.screen = Screen::AgentRun {
                           chunk_slug: chunk_slug.clone(),
                           lines: vec![],
                           scroll_offset: 0,
                           status: AgentRunStatus::Running,
                       };
                   }
               }
           }
       }
   }
   ```
   **Important implementation note on exit code:** `launch_agent_streaming` returns a `JoinHandle<i32>` that is stored on `App.agent_thread`. The bridge thread cannot join it (it would be a race). The cleaner solution: use two channels — `line_tx`/`line_rx` for lines (same as above), and a one-shot `exit_tx`/`exit_rx: mpsc::Receiver<i32>` for the exit code. Spawn a second thread that: joins `agent_thread` (moved out of `App` into the thread), gets the exit code, sends it via `exit_tx`. The bridge thread receives from `line_rx` (draining lines), then receives from `exit_rx` (one shot), then sends `TuiEvent::AgentDone { exit_code }`. Since the handle is now moved to the bridge thread, `App.agent_thread` is set to `None`. `handle_agent_done` then only refreshes milestones (no join needed). This is the cleaner design; implement it this way.

4. Update `handle_agent_done` to NOT attempt to join `agent_thread` (the T03 implementation joined it, but with the revised design above, the thread is moved into the bridge thread). Simplify: just refresh milestones and update status.

5. Run the full test suite and `just ready`:
   - `cargo test -p assay-tui` — ≥27+3 tests green
   - `cargo test -p assay-core --test pipeline_streaming` — still green
   - `just ready` — fmt + lint + test + deny all pass

## Must-Haves

- [ ] `assay-harness.workspace = true` in `crates/assay-tui/Cargo.toml`
- [ ] `App.event_tx: Option<mpsc::Sender<TuiEvent>>` field exists; set in `run()`
- [ ] `r` key in Dashboard: spawns agent only when `cycle_status()` returns `active_chunk_slug: Some(_)`, else no-op
- [ ] `r` key: transitions to `Screen::AgentRun { chunk_slug, lines: vec![], scroll_offset: 0, status: AgentRunStatus::Running }`
- [ ] Agent lines forwarded as `TuiEvent::AgentLine`; after agent stdout closes, `TuiEvent::AgentDone { exit_code }` sent
- [ ] Exit code from `JoinHandle<i32>` propagated to `AgentDone.exit_code` (not hardcoded 0)
- [ ] `agent_run_r_key_on_no_project_is_noop` test passes
- [ ] All 3 tests in `agent_run.rs` pass
- [ ] All 27+ existing TUI tests pass
- [ ] `just ready` passes

## Verification

- `cargo test -p assay-tui --test agent_run` — 3/3 pass
- `cargo test -p assay-tui` — 0 failures
- `cargo build -p assay-tui 2>&1 | grep "^error"` — empty
- `just ready 2>&1 | tail -5` — all checks green

## Observability Impact

- Signals added/changed: `App.event_tx` enables the `r` handler to wire agent output into the event loop; `TuiEvent::AgentLine(line)` / `TuiEvent::AgentDone { exit_code }` are the observable runtime signals for agent execution progress
- How a future agent inspects this: `Screen::AgentRun.lines` contains full agent stdout; `Screen::AgentRun.status` is `Done { exit_code }` or `Failed { exit_code }` after completion; accessible via `match &app.screen`
- Failure state exposed: Non-zero `exit_code` in `AgentRunStatus::Failed` is rendered in red in the status bar; `handle_agent_done` refreshes gate results so dashboard shows updated pass/fail counts immediately

## Inputs

- `crates/assay-tui/src/main.rs` — `TuiEvent` enum and channel from T03; needs `tx.clone()` to set `app.event_tx`
- `crates/assay-tui/src/app.rs` — `handle_agent_done` from T03 to update (no join needed); `handle_event` Dashboard arm to extend with `r` key
- `crates/assay-tui/tests/agent_run.rs` — T01 test file; `agent_run_r_key_on_no_project_is_noop` must now pass
- `crates/assay-core/src/pipeline.rs` — `launch_agent_streaming` from T02
- `crates/assay-core/src/pipeline.rs` — `build_harness_profile` (already exists)
- `crates/assay-harness/src/claude.rs` — `generate_config`, `write_config`, `build_cli_args` (already exists)

## Expected Output

- `crates/assay-tui/Cargo.toml` — `assay-harness` dependency added
- `crates/assay-tui/src/app.rs` — `App.event_tx` field; `r` key handler wired; `handle_agent_done` simplified
- `crates/assay-tui/src/main.rs` — `app.event_tx = Some(tx.clone())` set after channel creation
- All 3 `agent_run.rs` tests green
- `just ready` passes
