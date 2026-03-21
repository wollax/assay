---
id: T03
parent: S01
milestone: M007
provides:
  - channel-based event loop in main.rs (mpsc::Receiver<TuiEvent> replaces blocking event::read())
  - background crossterm thread spawning Key/Resize events into TuiEvent channel
  - r key handler in Dashboard arm that transitions to Screen::AgentRun and spawns relay-wrapper thread
  - assay-harness promoted to regular dependency in assay-tui (was dev-only)
key_files:
  - crates/assay-tui/src/main.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/Cargo.toml
key_decisions:
  - assay-harness moved from dev-dependency to regular dependency in assay-tui so app.rs can call claude::generate_config/write_config/build_cli_args in non-test code
  - r key handler returns false (no-op) when event_tx is None (test environments); all 8 agent_run tests bypass run() entirely so no wiring is needed in tests
  - relay-wrapper thread pattern: drain str_rx → join inner JoinHandle<i32> → send AgentDone; serialization ensures no AgentLine is lost before AgentDone
  - harness config written to std::env::temp_dir().join("assay-agent-{chunk_slug}") to avoid polluting worktree
patterns_established:
  - channel-based TUI event loop: background crossterm thread + mpsc::Receiver<TuiEvent> in main loop; AgentLine/AgentDone dispatched to App methods
  - relay-wrapper thread: inner launch_agent_streaming + outer str_rx drain + join + send AgentDone — guarantees ordering
observability_surfaces:
  - app.event_tx.is_some() → channel is wired (true at runtime, false in tests)
  - app.agent_thread.is_some() → relay-wrapper thread is live
  - app.screen discriminant Screen::AgentRun { status, lines } → runtime agent state
  - r key returns false silently if cycle_status returns None or write_config fails (screen stays Dashboard)
duration: 1 session
verification_result: passed
completed_at: 2026-03-21
blocker_discovered: false
---

# T03: Refactor `run()` to channel-based event loop + wire `r` key handler

**Replaced blocking `event::read()` loop with `mpsc::Receiver<TuiEvent>` dispatch loop, spawned crossterm background thread, and wired the `r` key in Dashboard to spawn the Claude agent relay-wrapper thread and transition to `Screen::AgentRun`.**

## What Happened

`main.rs::run()` was refactored from a blocking `crossterm::event::read()` loop to a channel-based `rx.recv()` loop over `mpsc::Receiver<TuiEvent>`. A background thread now reads raw crossterm events and sends `TuiEvent::Key` / `TuiEvent::Resize` into the channel. The main loop dispatches all four `TuiEvent` variants: `Key` → `handle_event`, `Resize` → `terminal.clear()`, `AgentLine` → `handle_agent_line`, `AgentDone` → `handle_agent_done`. `app.event_tx = Some(tx.clone())` is set before the loop so the `r` handler can clone the sender.

The `r` key handler was added to the `Screen::Dashboard` arm of `handle_event()`. It guards on `event_tx` being `Some` (no-op in tests), resolves the active chunk slug via `cycle_status`, builds a minimal `HarnessProfile`, writes harness config to a temp dir, transitions `self.screen` to `Screen::AgentRun`, and spawns the relay-wrapper thread. The relay-wrapper thread drains `str_rx` (sending `TuiEvent::AgentLine` for each line), then joins the inner `JoinHandle<i32>`, then sends `TuiEvent::AgentDone { exit_code }` — guaranteeing that all lines are emitted before `Done`.

`assay-harness` was promoted from a dev-dependency to a regular dependency in `assay-tui/Cargo.toml` so that `app.rs` can call `assay_harness::claude::generate_config/write_config/build_cli_args` in non-test code.

## Verification

```
cargo build -p assay-tui          → Finished (binary produced)
cargo test -p assay-tui           → 35 tests: 8 agent_run + 9 wizard + 6 help_status + 5 settings + 6 spec_browser + 1 app_wizard = all pass
cargo clippy -p assay-tui         → No warnings (pre-existing assay-types warning unrelated)
cargo test -p assay-tui --test agent_run -- r_key_noops_when_event_tx_is_none → ok
```

## Diagnostics

- `app.event_tx.is_some()` → `true` at runtime (wired in `run()`), `false` in test code (no channel)
- `app.agent_thread.is_some()` → active relay-wrapper thread in flight
- `app.screen` as `Screen::AgentRun { status: AgentRunStatus::Running, lines, .. }` → agent is streaming
- If `r` silently no-ops: check `cycle_status(&assay_dir)` returns `Some` and `write_config` succeeds
- Relay-wrapper panic → `rx.recv()` returns `Err(_)` → TUI exits gracefully (no crash)

## Deviations

None. Implementation followed the task plan exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/src/main.rs` — channel-based `run()` with background crossterm thread; `TuiEvent` dispatch loop
- `crates/assay-tui/src/app.rs` — `r` key handler in Dashboard arm; `assay_harness::claude` usage; `launch_agent_streaming` + `HarnessProfile` + `SettingsOverride` imports
- `crates/assay-tui/Cargo.toml` — `assay-harness` promoted from dev-dependency to regular dependency
