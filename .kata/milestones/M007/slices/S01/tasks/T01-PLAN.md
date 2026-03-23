---
estimated_steps: 5
estimated_files: 4
---

# T01: Write failing integration tests for streaming and AgentRun

**Slice:** S01 — Channel Event Loop and Agent Run Panel
**Milestone:** M007

## Description

Write the integration test files that define the exact API contracts for `launch_agent_streaming` and the `Screen::AgentRun` event flow. Tests compile (with minimal scaffolding stubs) but fail at runtime because the feature implementations are absent. This is the test-first anchor that prevents T02–T04 from drifting.

Two test files are produced:
1. `crates/assay-core/tests/pipeline_streaming.rs` — asserts on `launch_agent_streaming` line delivery and exit code
2. `crates/assay-tui/tests/agent_run.rs` — asserts on `App` state transitions driven by `AgentLine`/`AgentDone` events

Stub types needed to make tests compile are added to `app.rs`: `AgentRunStatus` enum, `Screen::AgentRun` variant, and `App::handle_agent_line` / `App::handle_agent_done` method stubs (body: `todo!()`). No functional implementation yet.

## Steps

1. Add stub type scaffolding to `crates/assay-tui/src/app.rs`:
   - `AgentRunStatus` enum: `Running`, `Done { exit_code: i32 }`, `Failed { exit_code: i32 }`
   - `Screen::AgentRun { chunk_slug: String, lines: Vec<String>, scroll_offset: usize, status: AgentRunStatus }` variant
   - `App.agent_thread: Option<std::thread::JoinHandle<i32>>` field (initialize to `None` in `with_project_root`)
   - `App::handle_agent_line(&mut self, line: String)` method stub: `todo!()`
   - `App::handle_agent_done(&mut self, exit_code: i32)` method stub: `todo!()`
   - Add `Screen::AgentRun { .. } => {}` arm to `draw()` match (empty arm to satisfy exhaustive match)

2. Write `crates/assay-core/tests/pipeline_streaming.rs` with three tests:
   - `streaming_delivers_lines_to_receiver`: spawns `sh -c 'printf "alpha\nbeta\ngamma\n"; exit 0'`, collects all lines from `line_rx`, asserts `["alpha", "beta", "gamma"]`
   - `streaming_join_handle_returns_exit_code`: spawns `sh -c 'exit 42'`, joins the handle, asserts exit code == 42
   - `streaming_failed_process_returns_nonzero`: spawns `sh -c 'echo err; exit 1'`, joins handle, asserts exit code == 1

3. Write `crates/assay-tui/tests/agent_run.rs` with three tests:
   - `agent_run_streams_lines_and_transitions_to_done`: construct `App::with_project_root(None)`, set `app.screen = Screen::AgentRun { chunk_slug: "test-chunk".into(), lines: vec![], scroll_offset: 0, status: AgentRunStatus::Running }`, call `app.handle_agent_line("line1".into())`, `app.handle_agent_line("line2".into())`, `app.handle_agent_done(0)`, assert `Screen::AgentRun { lines: ["line1", "line2"], status: Done { exit_code: 0 }, .. }`
   - `agent_run_failed_exit_code_shows_failed_status`: same setup, call `app.handle_agent_done(1)`, assert `status == AgentRunStatus::Failed { exit_code: 1 }`
   - `agent_run_r_key_on_no_project_is_noop`: construct `App::with_project_root(None)`, drive `handle_event(r key)`, assert screen is still `Screen::NoProject`

4. Verify test files compile (with stub types) by running `cargo check -p assay-core --tests` and `cargo check -p assay-tui --tests`

5. Run tests to confirm they fail at runtime (not compile time): `cargo test -p assay-core --test pipeline_streaming` and `cargo test -p assay-tui --test agent_run` — expect panics from `todo!()` or missing implementations

## Must-Haves

- [ ] `AgentRunStatus` enum and `Screen::AgentRun` variant exist in `app.rs` with correct field shapes
- [ ] `App.agent_thread` field exists (initialized to `None`)
- [ ] `handle_agent_line` and `handle_agent_done` method stubs compile
- [ ] `draw()` match arm for `Screen::AgentRun` exists (empty is fine)
- [ ] `pipeline_streaming.rs` has all three test functions with correct channel usage
- [ ] `agent_run.rs` has all three test functions with correct assertions
- [ ] `cargo check -p assay-core --tests` passes (no compile errors)
- [ ] `cargo check -p assay-tui --tests` passes (no compile errors)
- [ ] Tests fail at runtime (panics or assertion failures — not compile errors)

## Verification

- `cargo check -p assay-core --tests 2>&1 | tail -5` — expect "warning: unused ..." not "error[E...]"
- `cargo check -p assay-tui --tests 2>&1 | tail -5` — same expectation
- `cargo test -p assay-core --test pipeline_streaming 2>&1 | grep -E "FAILED|panicked"` — expect failures
- `cargo test -p assay-tui --test agent_run 2>&1 | grep -E "FAILED|panicked"` — expect failures

## Observability Impact

- Signals added/changed: `AgentRunStatus` and `Screen::AgentRun` types are now observable by integration tests via public `app.screen` field
- How a future agent inspects this: `match &app.screen { Screen::AgentRun { lines, status, .. } => ... }` — status and line buffer readable directly
- Failure state exposed: `AgentRunStatus::Failed { exit_code }` captures non-zero exits; `AgentRunStatus::Done { exit_code }` captures successful exits

## Inputs

- `crates/assay-tui/src/app.rs` — existing `Screen` enum and `App` struct to extend with new variants/fields
- `crates/assay-core/src/pipeline.rs` — signature reference for `launch_agent_streaming` (not yet implemented, but types inform test imports)
- `S01-PLAN.md` — authoritative field names and type shapes for `AgentRunStatus`, `Screen::AgentRun`

## Expected Output

- `crates/assay-tui/src/app.rs` — extended with stub scaffolding (compiles, no impl)
- `crates/assay-core/tests/pipeline_streaming.rs` — new file with 3 failing tests
- `crates/assay-tui/tests/agent_run.rs` — new file with 3 failing tests
