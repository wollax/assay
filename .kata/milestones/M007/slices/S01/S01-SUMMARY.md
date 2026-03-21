---
id: S01
parent: M007
milestone: M007
provides:
  - TuiEvent enum in assay_tui::app (Key, Resize, AgentLine, AgentDone)
  - AgentRunStatus enum in assay_tui::app (Running, Done { exit_code }, Failed { exit_code })
  - Screen::AgentRun { chunk_slug, lines, scroll_offset, status } variant
  - App.event_tx and App.agent_thread fields (None by default, wired at runtime)
  - App::handle_agent_line (cap-at-10k, no-op on non-AgentRun)
  - App::handle_agent_done (Done/Failed transition + disk refresh)
  - draw_agent_run free function (scrollable list, status bar, Starting... placeholder)
  - Screen::AgentRun handle_event arm (Esc → Dashboard, j/k/↑/↓ scroll)
  - launch_agent_streaming in assay_core::pipeline (BufReader::lines, JoinHandle<i32>)
  - channel-based run() in main.rs (mpsc::Receiver<TuiEvent>, background crossterm thread)
  - r key handler in Dashboard arm (harness config → relay-wrapper thread → Screen::AgentRun)
  - crates/assay-tui/tests/agent_run.rs with 8 integration tests (all green)
requires: []
affects:
  - slice: S02
    provides: TuiEvent loop + Screen::AgentRun + App.config available at r-key spawn time
  - slice: S03
    provides: TuiEvent loop (/ key can arrive from any screen)
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/src/main.rs
  - crates/assay-core/src/pipeline.rs
  - crates/assay-tui/Cargo.toml
  - crates/assay-tui/tests/agent_run.rs
key_decisions:
  - D107 — unified mpsc channel combining terminal events and agent output
  - D108 — launch_agent_streaming new free function; existing launch_agent unchanged
  - D112 — AgentRunStatus (not AgentStatus) to avoid collision with assay-core::checkpoint
  - D113 — relay-wrapper thread: drain str_rx → join inner JoinHandle → send AgentDone (no line loss)
  - D114 — harness config written to temp_dir/assay-agent-{slug}/ for S01 MVP r key handler
patterns_established:
  - launch_agent_streaming drops line_tx before child.wait() so receiver sees EOF before thread blocks on wait
  - relay-wrapper thread serializes drain → join → done, guaranteeing AgentLine ordering before AgentDone
  - cap-at-10k via Vec push + remove(0); simple and sufficient
  - channel-based TUI event loop: background crossterm thread + mpsc::Receiver<TuiEvent> in main loop
observability_surfaces:
  - app.event_tx.is_some() → channel is wired (true at runtime, false in tests)
  - app.agent_thread.is_some() → relay-wrapper thread is live
  - Screen::AgentRun { lines, status } → lines holds full stdout buffer (capped at 10 000); status holds final exit code
  - TUI status bar renders "● Running…", "✓ Done (exit 0)", or "✗ Failed (exit N)"
  - r key handler returns false silently if cycle_status returns None or write_config fails
drill_down_paths:
  - .kata/milestones/M007/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M007/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M007/slices/S01/tasks/T03-SUMMARY.md
  - .kata/milestones/M007/slices/S01/tasks/T04-SUMMARY.md
duration: ~3h (4 tasks across one session)
verification_result: passed
completed_at: 2026-03-21
---

# S01: Channel Event Loop and Agent Run Panel

**Replaced the blocking `event::read()` TUI loop with a channel-based `TuiEvent` dispatch loop, added `launch_agent_streaming` to assay-core, implemented `Screen::AgentRun` with live line accumulation and Done/Failed status, and wired the `r` key in Dashboard to spawn the Claude agent relay-wrapper thread — all 35 assay-tui tests pass and `just ready` is green.**

## What Happened

T01 laid all type contracts: `TuiEvent`, `AgentRunStatus`, `Screen::AgentRun` with stub draw/event arms, `App.event_tx` / `App.agent_thread` fields, stub methods `handle_agent_line` / `handle_agent_done`, and the real `launch_agent_streaming` in `pipeline.rs`. The 8-test scaffold in `agent_run.rs` was written first; the two subprocess tests (`launch_agent_streaming_delivers_all_lines`, `delivers_exit_code`) passed immediately while the App state machine tests failed as intended. The key pattern established: `launch_agent_streaming` drops `line_tx` before `child.wait()` so the receiver sees EOF before the thread blocks on the process.

T02 closed the App state machine loop: `handle_agent_line` pushes to `Screen::AgentRun.lines` with cap-at-10 000 (Vec push + remove(0)); `handle_agent_done` transitions status to `Done { exit_code }` / `Failed { exit_code }` then refreshes `milestones` and `cycle_slug` from disk with `.ok()` graceful degradation; `draw_agent_run` renders a bordered Block with a scrollable line list, a "Starting…" placeholder when lines is empty, and a color-coded status bar. All 8 agent_run tests passed.

T03 wired the runtime: `run()` in `main.rs` was refactored from blocking `crossterm::event::read()` to an `mpsc::channel::<TuiEvent>()` loop with a background crossterm thread. The `r` key handler in the Dashboard arm guards on `event_tx` being `Some`, calls `cycle_status` for the active chunk, builds a minimal `HarnessProfile`, writes harness config to `temp_dir/assay-agent-{slug}/`, spawns the relay-wrapper thread (drain `str_rx` → join inner `JoinHandle<i32>` → send `TuiEvent::AgentDone`), and transitions to `Screen::AgentRun`. `assay-harness` was promoted from dev-dependency to regular dependency so `app.rs` can call the adapter functions in non-test code.

T04 confirmed the workspace was already clean from T01–T03: `cargo fmt`, `cargo clippy`, `cargo test --workspace`, and `cargo deny check` all passed without any fixes.

## Verification

```
cargo test -p assay-tui --test agent_run    →  8/8 pass
cargo test -p assay-tui                     →  35/35 pass (27 pre-existing + 8 agent_run)
cargo test -p assay-core -- launch_agent_streaming  →  2/2 pass
cargo build -p assay-tui                    →  binary produced (14.8 MB)
just ready                                  →  exit 0 (fmt, clippy, test, deny all green)
```

## Requirements Advanced

- R053 (TUI agent spawning) — S01 proves the core spawn/stream/display loop: real echo subprocess → `launch_agent_streaming` → `TuiEvent::AgentLine` → `Screen::AgentRun.lines` accumulation → `AgentRunStatus::Done`. Real Claude invocation is UAT-only.
- R054 (Provider abstraction) — S01 wires the Anthropic (Claude Code) path end-to-end in the `r` key handler. S02 adds Ollama and OpenAI dispatch.

## Requirements Validated

- R053 — partially validated; full validation requires S02 provider wiring and human UAT with a real project.
- R054 — partially validated; Anthropic path proven; Ollama/OpenAI paths proven in S02.

## New Requirements Surfaced

- None.

## Requirements Invalidated or Re-scoped

- None.

## Deviations

- `launch_agent_streaming` unit tests added to `pipeline.rs` (in addition to `agent_run.rs`) to satisfy the slice-level verification command `cargo test -p assay-core -- launch_agent_streaming`. The plan placed all 8 tests in the TUI test file; the verification section required the assay-core command independently. Both now exist.
- `assay-harness` promoted from dev-dependency to regular dependency in T03 (not called out explicitly in the T03 plan, but required for `app.rs` non-test code to call adapter functions).

## Known Limitations

- The `r` key handler writes harness config to `temp_dir/assay-agent-{slug}/` — the subprocess may not have access to the project worktree. Real worktree launch is S02's responsibility (D114).
- Provider dispatch in S01 is Anthropic-only. Ollama and OpenAI are added in S02.
- Real Claude invocation is UAT-only; no automated test drives a real model.

## Follow-ups

- S02: replace `temp_dir` harness config path with a real worktree path from the pipeline; add Ollama and OpenAI provider dispatch via `provider_harness_writer`.
- S03: add slash command overlay using the `TuiEvent` loop established here.
- S04: add MCP server configuration panel (independent — no dependency on S01's event loop).

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — `TuiEvent`, `AgentRunStatus`, `Screen::AgentRun`, `event_tx`/`agent_thread` fields, `handle_agent_line`, `handle_agent_done`, `draw_agent_run`, updated `draw()` and `handle_event()`, `r` key handler in Dashboard arm
- `crates/assay-tui/src/main.rs` — channel-based `run()` with background crossterm thread; `TuiEvent` dispatch loop
- `crates/assay-core/src/pipeline.rs` — `launch_agent_streaming` + 2 unit tests
- `crates/assay-tui/Cargo.toml` — `assay-harness` promoted from dev-dep to regular dep
- `crates/assay-tui/tests/agent_run.rs` — new file: 8 integration tests

## Forward Intelligence

### What the next slice should know
- `App.event_tx` is `None` in all existing integration tests — the `r` key handler and any new handlers that push to the channel must guard on `event_tx.is_some()` and return false silently, otherwise tests that construct `App::with_project_root()` without calling `run()` will panic.
- `Screen::AgentRun` arms in both `draw()` and `handle_event()` exist and are stable. S02's provider dispatch extension point is the `r` key handler in `handle_event()`'s Dashboard arm — add `provider_harness_writer(&self.config)` call there to swap between Claude/Ollama/OpenAI.
- The relay-wrapper thread pattern (D113) must not be changed — any change that lets `AgentDone` arrive before the last `AgentLine` will cause the status bar to flash "Done" while lines are still arriving.

### What's fragile
- The `r` key handler writes to `temp_dir` — if two `r` presses happen in rapid succession for the same chunk slug, the second will overwrite the first's harness config while it's running. S02 should use a timestamped or uuid-suffixed run directory.
- `handle_agent_done` refreshes `milestones` and `cycle_slug` synchronously on the main event loop thread. If the project has many milestones, this could cause a brief frame skip. Acceptable for now (D091 rationale).

### Authoritative diagnostics
- `app.event_tx.is_some()` — first check when `r` key does nothing: if None, `run()` was not called (test context or startup bug)
- `app.screen` discriminant — if stuck on Dashboard after `r`, check `cycle_status` returns `Some` and `write_config` succeeded (both degrade silently in S01)
- TUI status bar "● Running…" / "✓ Done" / "✗ Failed" — visible without code inspection

### What assumptions changed
- The plan assumed `assay-harness` would stay a dev-dependency through S01. Promoting it to a regular dependency was necessary in T03 to call adapter functions in `app.rs` non-test code.
- The plan listed `launch_agent_streaming` tests only in `agent_run.rs`. The slice-level verification command required them in `pipeline.rs` too.
