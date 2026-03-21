---
id: T01
parent: S01
milestone: M007
provides:
  - TuiEvent enum in assay_tui::app
  - AgentRunStatus enum in assay_tui::app
  - Screen::AgentRun variant with stub draw/event arms
  - App.event_tx and App.agent_thread fields (None by default)
  - App::handle_agent_line and App::handle_agent_done stub methods
  - launch_agent_streaming in assay_core::pipeline
  - crates/assay-tui/tests/agent_run.rs with 8 integration tests (4 pass, 4 red)
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-core/src/pipeline.rs
  - crates/assay-tui/Cargo.toml
  - crates/assay-tui/tests/agent_run.rs
key_decisions:
  - launch_agent_streaming uses unbounded mpsc::channel() to avoid deadlock
  - launch_agent_streaming tests added to pipeline.rs unit tests (not only TUI tests) to satisfy cargo test -p assay-core -- launch_agent_streaming verification command
  - TuiEvent and AgentRunStatus defined in app.rs (lib crate), not main.rs
patterns_established:
  - launch_agent_streaming drops line_tx before child.wait() to ensure receiver sees EOF before thread blocks
  - Screen::AgentRun stub draw uses draw_agent_run_stub helper (full impl deferred to T02)
observability_surfaces:
  - app.event_tx.is_some() indicates whether channel is wired
  - app.screen variant indicates current state (AgentRun vs Dashboard etc.)
  - Screen::AgentRun.status exposes Running/Done/Failed at all times
duration: ~45min
verification_result: partial (T01 tasks 1-2 pass; tasks 3-8 red as expected; 27 pre-existing tests green)
completed_at: 2026-03-21
blocker_discovered: false
---

# T01: Define `TuiEvent`, `AgentRunStatus`, `Screen::AgentRun`, and `launch_agent_streaming` — with failing tests

**Added all S01 type contracts to `app.rs` and `pipeline.rs`, wrote 8-test scaffold (4 green / 4 red as expected).**

## What Happened

Implemented all 7 steps from the task plan:

1. Added `TuiEvent` and `AgentRunStatus` enums to `app.rs` above the `Screen` enum, with `use std::sync::mpsc` import.

2. Added `Screen::AgentRun { chunk_slug, lines, scroll_offset, status }` variant. Added `draw_agent_run_stub` helper (renders bordered block with status and line count). Added `Screen::AgentRun { .. } => false` arm in `handle_event`.

3. Added `event_tx: Option<mpsc::Sender<TuiEvent>>` and `agent_thread: Option<std::thread::JoinHandle<i32>>` fields to `App`, both initialized to `None` in `with_project_root`.

4. Added stub methods `pub fn handle_agent_line(&mut self, _line: String) {}` and `pub fn handle_agent_done(&mut self, _exit_code: i32) {}` — intentional no-ops for T01.

5. Implemented `launch_agent_streaming` in `assay-core::pipeline`. Uses `BufReader::lines()` to drain stdout, sends each line via `line_tx`, drops `line_tx` before `child.wait()` (so receiver sees EOF before thread blocks), returns `JoinHandle<i32>` with the exit code.

6. Added `assay-harness.workspace = true` to `[dev-dependencies]` in `crates/assay-tui/Cargo.toml`.

7. Wrote `crates/assay-tui/tests/agent_run.rs` with all 8 tests. Tests 1–2 (real subprocess via `sh -c printf` and `true`/`false`) pass immediately. Tests 3–8 (App state machine stubs) fail as expected with clear assertion messages.

**Deviation from plan:** Added `launch_agent_streaming` unit tests directly in `pipeline.rs` (in addition to the TUI test file) to satisfy the slice verification command `cargo test -p assay-core -- launch_agent_streaming`. The plan didn't explicitly call this out but the verification section requires it.

## Verification

```
cargo check -p assay-tui          → clean (0 errors, 0 warnings)
cargo test -p assay-core -- launch_agent_streaming → 2 passed
cargo test -p assay-tui           → 27 pre-existing pass; 4 new pass; 4 new fail (expected)
```

Pre-existing tests by file:
- app_wizard.rs: 1/1 ✓
- help_status.rs: 6/6 ✓
- settings.rs: 5/5 ✓
- spec_browser.rs: 6/6 ✓
- wizard_round_trip.rs: 9/9 ✓

New agent_run.rs results:
- launch_agent_streaming_delivers_all_lines ✓
- launch_agent_streaming_delivers_exit_code ✓
- handle_agent_line_noops_on_non_agent_run_screen ✓
- r_key_noops_when_event_tx_is_none ✓
- handle_agent_line_accumulates_in_agent_run_screen ✗ (stub no-op, fixed in T02)
- handle_agent_done_zero_exit_transitions_to_done ✗ (stub no-op, fixed in T02)
- handle_agent_done_nonzero_exit_transitions_to_failed ✗ (stub no-op, fixed in T02)
- handle_agent_line_caps_at_ten_thousand ✗ (stub no-op, fixed in T02)

## Diagnostics

- `app.event_tx.is_some()` → whether TUI channel is wired (false until main.rs T03 wiring)
- `app.agent_thread.is_some()` → whether a subprocess is tracked
- `app.screen` discriminant → current screen (AgentRun visible)
- `launch_agent_streaming` drops `line_tx` on spawn error → receiver channel closes → relay thread sees disconnect → can emit `AgentDone { exit_code: -1 }`

## Deviations

Added unit tests for `launch_agent_streaming` inside `pipeline.rs` test module to satisfy the slice-level verification command `cargo test -p assay-core -- launch_agent_streaming`. The task plan placed all 8 tests in `assay-tui/tests/agent_run.rs`, but the verification section separately mandated the assay-core command. Both locations now have the launch_agent_streaming tests.

## Known Issues

Tests 3–6 in `agent_run.rs` fail intentionally (stub methods). They will pass after T02 implements `handle_agent_line` and `handle_agent_done`.

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — Added `TuiEvent`, `AgentRunStatus`, `Screen::AgentRun`, `event_tx`/`agent_thread` fields, `handle_agent_line`/`handle_agent_done` stubs, `draw_agent_run_stub`
- `crates/assay-core/src/pipeline.rs` — Added `launch_agent_streaming` + 2 unit tests
- `crates/assay-tui/Cargo.toml` — Added `assay-harness` dev-dependency
- `crates/assay-tui/tests/agent_run.rs` — New file: 8 integration tests (2 pass, 4 red, 2 pass)
