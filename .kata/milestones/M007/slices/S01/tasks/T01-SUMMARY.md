---
id: T01
parent: S01
milestone: M007
provides:
  - AgentRunStatus enum and Screen::AgentRun variant in app.rs
  - App.agent_thread field stub
  - App::handle_agent_line and App::handle_agent_done method stubs (todo!())
  - launch_agent_streaming stub in pipeline.rs
  - crates/assay-core/tests/pipeline_streaming.rs (3 failing tests)
  - crates/assay-tui/tests/agent_run.rs (3 tests: 2 failing, 1 passing)
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-core/src/pipeline.rs
  - crates/assay-core/tests/pipeline_streaming.rs
  - crates/assay-tui/tests/agent_run.rs
key_decisions:
  - launch_agent_streaming stub returns todo!() — allows tests to compile; real impl in T02
  - Screen::AgentRun arm in handle_event quits on q/Esc (minimal safe default)
  - AgentRunStatus derives PartialEq+Eq to enable assert_eq! in tests
patterns_established:
  - Test-first anchor pattern — integration tests compile against stub types and fail at runtime
  - Screen::AgentRun { chunk_slug, lines, scroll_offset, status } field layout established
  - AgentRunStatus::Running/Done{exit_code}/Failed{exit_code} enum shape established
observability_surfaces:
  - App.screen readable as Screen::AgentRun { status, lines, .. } from integration tests
  - AgentRunStatus::Done { exit_code } and Failed { exit_code } expose subprocess outcome
duration: ~20 minutes
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T01: Write failing integration tests for streaming and AgentRun

**Added stub types and two integration-test files that compile cleanly but fail at runtime, establishing the T02–T04 API contract.**

## What Happened

Added stub scaffolding to `crates/assay-tui/src/app.rs`:
- `AgentRunStatus` enum (`Running`, `Done { exit_code: i32 }`, `Failed { exit_code: i32 }`) with `PartialEq + Eq` derives for test assertions
- `Screen::AgentRun { chunk_slug, lines, scroll_offset, status }` variant
- `App.agent_thread: Option<std::thread::JoinHandle<i32>>` field initialized to `None`
- `handle_agent_line` and `handle_agent_done` method stubs with `todo!()`
- `Screen::AgentRun { .. }` arm in `draw()` (empty) and `handle_event()` (q/Esc exits)

Added `launch_agent_streaming` stub to `crates/assay-core/src/pipeline.rs` returning `todo!()` to satisfy test imports.

Wrote `crates/assay-core/tests/pipeline_streaming.rs` with three tests targeting `launch_agent_streaming` channel delivery and exit-code reporting.

Wrote `crates/assay-tui/tests/agent_run.rs` with three tests: two verify `handle_agent_line`/`handle_agent_done` state transitions, one verifies that `r` key on `Screen::NoProject` is a no-op (passes already, as intended).

## Verification

```
cargo check -p assay-core --tests    # → Finished, zero errors
cargo check -p assay-tui --tests     # → Finished, zero errors (2 harmless warnings fixed)
cargo test -p assay-core --test pipeline_streaming
  # → 3 FAILED (panicked at pipeline.rs:332 — todo!() in launch_agent_streaming)
cargo test -p assay-tui --test agent_run
  # → 2 FAILED (panicked at app.rs — todo!() in handle_agent_line/done), 1 PASSED (noop test)
cargo test -p assay-tui --test app_wizard --test help_status --test settings --test spec_browser --test wizard_round_trip
  # → 27 existing tests all pass
```

## Diagnostics

- `match &app.screen { Screen::AgentRun { lines, status, .. } => ... }` — read line buffer and status after driving events
- `AgentRunStatus::Done { exit_code }` / `Failed { exit_code }` — subprocess exit code surfaced in enum
- Stub panics include file+line: `pipeline.rs:332` and `app.rs:614/622` for easy location

## Deviations

- Added `Screen::AgentRun` arm to `handle_event` (not mentioned in plan) — required for exhaustive match compilation; implemented as q/Esc → exit with `false` otherwise, which is the safest default.

## Known Issues

- None. All expected outcomes achieved.

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — added `AgentRunStatus` enum, `Screen::AgentRun` variant, `App.agent_thread` field, `handle_agent_line`/`handle_agent_done` stubs, draw and event arms
- `crates/assay-core/src/pipeline.rs` — added `launch_agent_streaming` stub
- `crates/assay-core/tests/pipeline_streaming.rs` — new file with 3 failing integration tests
- `crates/assay-tui/tests/agent_run.rs` — new file with 3 integration tests (2 failing, 1 passing)
