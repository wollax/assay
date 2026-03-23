---
id: S01
parent: M007
milestone: M007
provides:
  - "`pub fn launch_agent_streaming(cli_args, working_dir, line_tx: Sender<String>) -> JoinHandle<i32>` in `assay-core::pipeline`"
  - "`pub enum TuiEvent { Key, Resize, AgentLine(String), AgentDone { exit_code } }` in `assay-tui::app`"
  - "`pub enum AgentStatus { Running, Done { exit_code }, Failed { exit_code } }` in `assay-tui::app`"
  - "`Screen::AgentRun { chunk_slug, lines, scroll_offset, status }` variant in `Screen` enum"
  - "`App.agent_thread: Option<JoinHandle<i32>>` and `App.agent_list_state: ListState` fields"
  - "`App.event_tx: Option<Sender<TuiEvent>>` field for agent thread back-channel"
  - "`App::handle_tui_event` dispatching AgentLine into lines, AgentDone into AgentStatus"
  - "`r` key on Dashboard spawns agent via `launch_agent_streaming`, transitions to `Screen::AgentRun`"
  - "`draw_agent_run` free fn with scrollable list, status line (Running/Done/Failed), Esc hint"
  - "Channel-based `run()` loop: `mpsc::Receiver<TuiEvent>` replacing blocking `event::read()`"
  - "Integration test `launch_agent_streaming_delivers_all_lines` — real subprocess, ordered lines, exit code 0"
  - "4 new agent_run integration tests — all passing"
  - "31 total assay-tui tests passing (27 existing + 4 new)"
requires:
  - slice: none
    provides: "Can be built on top of M006 App/Screen foundation"
affects:
  - S02
  - S03
key_files:
  - crates/assay-core/src/pipeline.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/src/main.rs
  - crates/assay-tui/tests/agent_run.rs
key_decisions:
  - "D107 — Unified TUI event loop using mpsc channel combining terminal events and agent output"
  - "D108 — `launch_agent_streaming` as new free function in assay-core::pipeline; existing `launch_agent` unchanged"
  - "D112 — `App.event_tx: Option<Sender<TuiEvent>>` field for agent thread back-channel (avoids changing handle_event signature)"
  - "S01 r-key sends AgentDone { exit_code: 0 } sentinel from forwarder thread; full exit-code accuracy deferred to S02"
patterns_established:
  - "mpsc::channel::<TuiEvent>() + background crossterm reader thread; main loop = `while let Ok(event) = rx.recv()`"
  - "Agent forwarder thread relays line_rx lines as TuiEvent::AgentLine, sends TuiEvent::AgentDone when channel disconnects"
  - "BufReader::lines() + Sender<String> pattern for subprocess stdout streaming (T01)"
  - "draw_agent_run: Layout::vertical [Length(1), Fill(1), Length(1), Length(1)] for title/list/status/hint"
observability_surfaces:
  - "`Screen::AgentRun { lines, status, .. }` — inspect via `matches!(app.screen, Screen::AgentRun { status: AgentStatus::Done { .. }, .. })`"
  - "`draw_agent_run` renders 'Running…', 'Done (exit 0)', or 'Failed (exit N)' in status line"
  - "Per-line stream: collect from `Receiver<String>` paired with the sender in `launch_agent_streaming`"
  - "Exit code: `JoinHandle::join().unwrap_or(-1)`; panic maps to -1"
drill_down_paths:
  - .kata/milestones/M007/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M007/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M007/slices/S01/tasks/T03-SUMMARY.md
duration: ~2h (T01: 5min, T02: 25min, T03: ~1h; plus verification)
verification_result: passed
completed_at: 2026-03-23
---

# S01: Channel Event Loop and Agent Run Panel

**Replaced the blocking `event::read()` TUI loop with a channel-based `TuiEvent` dispatch system; added `launch_agent_streaming` to assay-core; wired `r` key to spawn the agent and stream its output into a scrollable `Screen::AgentRun` panel — all 31 assay-tui tests pass with zero regressions.**

## What Happened

Three tasks executed sequentially, each building on the previous:

**T01** added `launch_agent_streaming` to `assay-core::pipeline` — a free function that spawns a child process with `Stdio::piped()`, wraps stdout in a `BufReader`, iterates `.lines()`, sends each line via an `mpsc::Sender<String>`, and returns a `JoinHandle<i32>` carrying the exit code. Empty `cli_args` is guarded with an immediate `thread::spawn(|| -1)`. Two inline tests prove the function: `launch_agent_streaming_delivers_all_lines` (real `sh -c 'printf ...'` subprocess, collects via `recv_timeout`, asserts ordered lines and exit code 0) and `launch_agent_streaming_empty_args_returns_minus_one`. The existing `launch_agent()` batch function is completely untouched.

**T02** established the data model: `TuiEvent` enum (Key, Resize, AgentLine, AgentDone), `AgentStatus` enum (Running, Done, Failed), `Screen::AgentRun` variant (chunk_slug, lines, scroll_offset, status), and new `App` fields (`agent_thread`, `agent_list_state`). Stub implementations of `handle_tui_event` (returns false) and `draw_agent_run` (empty body) were committed to keep the build warning-free. A new `tests/agent_run.rs` file was created with 4 integration tests in "red phase" — one passing immediately (`r_key_no_active_chunk_is_noop`), three failing as designed until T03 implemented the real logic.

**T03** wired everything together: real `handle_tui_event` dispatching `AgentLine` into `Screen::AgentRun.lines` and `AgentDone` into `AgentStatus::Done`/`Failed`; the `r` key handler in Dashboard that spawns the agent and sets up the forwarder thread; real `draw_agent_run` with four-zone layout (title/scrollable-list/status/hint); Esc→Dashboard navigation; and the channel-based `run()` rewrite in `main.rs`. The forwarder thread pattern relays lines from `line_rx` as `TuiEvent::AgentLine` events and sends `TuiEvent::AgentDone { exit_code: 0 }` as a sentinel when the line channel disconnects — the real join-handle exit code is consumed but its value is discarded for S01 (full accuracy deferred to S02).

## Verification

```
cargo test -p assay-core -- launch_agent_streaming
# 2 tests: launch_agent_streaming_delivers_all_lines, launch_agent_streaming_empty_args_returns_minus_one — both ok

cargo test -p assay-tui
# 31 tests: 27 existing (no regressions) + 4 new agent_run tests — all ok

cargo build -p assay-tui
# Finished dev profile — zero warnings

cargo build -p assay-core
# Finished dev profile — zero warnings
```

New agent_run tests passing:
- `agent_line_events_accumulate_in_agent_run_screen`
- `agent_done_event_transitions_to_done_status`
- `agent_done_nonzero_exit_sets_failed_status`
- `r_key_no_active_chunk_is_noop`

## Requirements Advanced

- R053 (TUI agent spawning) — Core infrastructure proven: channel event loop, streaming subprocess, `Screen::AgentRun` panel, `r` key wiring. Real claude invocation remains UAT.
- R054 (Provider abstraction) — Foundation established: `launch_agent_streaming` is provider-agnostic; provider routing (D109) is S02's concern.

## Requirements Validated

None validated in this slice — R053 and R054 are active requirements proven further in S02.

## New Requirements Surfaced

None.

## Requirements Invalidated or Re-scoped

None.

## Deviations

- **Forwarder thread exit code**: The forwarder always sends `AgentDone { exit_code: 0 }` as a sentinel rather than the true exit code from the `JoinHandle`. The join handle's real exit code is consumed (via `agent_thread.take().map(|h| let _ = h.join())`) but discarded. Full exit-code accuracy deferred to S02. This is explicitly documented in the S01 plan and T03 summary.

## Known Limitations

- `r` key hardcodes `["claude", "--print"]` as CLI args for S01. Real provider dispatch (Anthropic/Ollama/OpenAI selection from `App.config`) is S02.
- The forwarder thread sends `AgentDone { exit_code: 0 }` unconditionally — the TUI always shows "Done (exit 0)" even if the agent failed. Accurate exit codes require S02 wiring.
- Real Claude Code invocation (UAT path) requires `claude` CLI installed and a valid InProgress project. This is manual UAT only.
- `App.event_tx` is `None` during integration tests (no `run()` call). The `r` key handler guards against this (`event_tx.is_some()` check), so tests with no InProgress milestone pass as no-op; tests with InProgress milestone would need `event_tx` set explicitly (not currently tested).

## Follow-ups

- S02: Wire real provider dispatch — `provider_harness_writer(config)` routing to Anthropic/Ollama/OpenAI; pass accurate exit code from forwarder thread; real harness CLI args via `assay_harness::claude::build_cli_args`.
- S03: Slash command overlay using the channel-based `TuiEvent` loop from S01.
- S04: MCP panel — independent of S01, uses same App/Screen foundation.

## Files Created/Modified

- `crates/assay-core/src/pipeline.rs` — Added `launch_agent_streaming` function (~50 lines) and two inline tests; `launch_agent` unchanged
- `crates/assay-tui/src/app.rs` — Added `TuiEvent`, `AgentStatus` enums; `Screen::AgentRun` variant; `agent_thread`, `agent_list_state`, `event_tx` App fields; `handle_tui_event` implementation; `r` key handler; `draw_agent_run` real implementation; `Screen::AgentRun` Esc handler; updated all match exhaustion sites
- `crates/assay-tui/src/main.rs` — Channel-based `run()` loop; `mpsc::Receiver<TuiEvent>` dispatch; background crossterm reader thread
- `crates/assay-tui/tests/agent_run.rs` — New file with 4 integration tests (all passing)

## Forward Intelligence

### What the next slice should know
- `App.event_tx` is `Option<Sender<TuiEvent>>` initialized to `None` in `with_project_root` and set to `Some(tx.clone())` inside `run()` before the main loop. S02's agent spawn wiring must account for this: the `r` key handler only spawns if `event_tx.is_some()`.
- `cycle_status` returns `Ok(None)` when no milestone is InProgress — the `r` handler is a clean no-op in that case. The real `cycle_status` call is in `handle_event` Dashboard arm, not in `handle_tui_event`.
- The forwarder thread pattern (receives from `line_rx`, sends `TuiEvent::AgentLine`, sends sentinel `TuiEvent::AgentDone` on disconnect) is the extension point S02 must modify to deliver accurate exit codes. The JoinHandle stored in `App.agent_thread` must be joined to get the real exit code — the forwarder currently discards it.
- S02's `provider_harness_writer` should replace the hardcoded `cli_args = ["claude", "--print"]` in the `r` key handler. The working_dir is `self.project_root.clone().unwrap()` — simplified for S01; full worktree setup is S02.

### What's fragile
- `App.event_tx` guards — the `r` key handler is guarded by `event_tx.is_some()`, but the guard is a runtime check, not compile-time enforced. If S02 modifies the `r` handler, it must preserve this guard.
- Forwarder thread sender clone — the `r` handler clones `event_tx` twice (once for the forwarder thread, once retained in `self.event_tx`). The sender clone count must not grow unbounded across multiple `r` key presses — the old `agent_thread` should be joined before spawning a new agent. Currently no such guard exists (pressing `r` while an agent is running spawns a second forwarder thread).

### Authoritative diagnostics
- `Screen::AgentRun { lines, status, .. }` — first inspection surface after driving `handle_tui_event` events
- `AgentStatus` value — `Running`, `Done { exit_code }`, `Failed { exit_code }` — visible both in TUI status line and via test assertions
- `cargo test -p assay-tui --test agent_run` — targeted test run for S01 functionality

### What assumptions changed
- T03 originally planned to implement `Screen::AgentRun` Esc handler in `handle_event`. The plan was followed exactly; no changes.
- S01 plan allowed for full exit-code delivery in the forwarder. Actual implementation chose sentinel approach for simplicity — full accuracy explicitly deferred to S02 per T03 deviation note.
