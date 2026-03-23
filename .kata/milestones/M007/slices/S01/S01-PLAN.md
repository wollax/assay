# S01: Channel Event Loop and Agent Run Panel

**Goal:** Replace the blocking `event::read()` loop in `main.rs` with a channel-based `mpsc::Receiver<TuiEvent>` loop; add `launch_agent_streaming` to `assay-core::pipeline`; introduce `Screen::AgentRun` with `AgentStatus` and `draw_agent_run`; wire the `r` key on the Dashboard to spawn the agent (Anthropic/claude hardcoded for S01) and display its live output. All 27 existing TUI tests continue to pass.

**Demo:** Integration tests prove (1) `launch_agent_streaming` delivers all stdout lines from a real echo subprocess before the thread joins, and (2) a synthetic `TuiEvent::AgentLine` / `TuiEvent::AgentDone` event sequence drives `App` to accumulate lines in `Screen::AgentRun` and transition to `Done` status — no real terminal required. `cargo test -p assay-tui` and `cargo test -p assay-core` both pass.

## Must-Haves

- `launch_agent_streaming(cli_args, working_dir, line_tx: Sender<String>) -> JoinHandle<i32>` exists in `assay-core::pipeline`; an integration test spawns `sh -c 'echo line1; echo line2; exit 0'` and asserts all lines received and exit code 0
- `TuiEvent` enum with `Key(KeyEvent)`, `Resize(u16, u16)`, `AgentLine(String)`, `AgentDone { exit_code: i32 }` variants in `assay-tui::app` (or `main.rs`)
- `run()` in `main.rs` refactored to `mpsc::Receiver<TuiEvent>`; one background thread feeds `Key`/`Resize`; agent thread (when spawned) feeds `AgentLine`/`AgentDone`
- `Screen::AgentRun { chunk_slug: String, lines: Vec<String>, scroll_offset: usize, status: AgentStatus }` variant added to `Screen` enum
- `AgentStatus` enum: `Running`, `Done { exit_code: i32 }`, `Failed { exit_code: i32 }`
- `App.agent_thread: Option<JoinHandle<i32>>` field initialized to `None` in `with_project_root`; consumed on `AgentDone`
- `App.agent_list_state: ListState` field for scrollable agent output
- `r` key handler in Dashboard arm: calls `cycle_status` to get `active_chunk_slug`; if `None`, is a no-op; otherwise constructs hardcoded claude CLI args and calls `launch_agent_streaming`, transitions to `Screen::AgentRun`
- `draw_agent_run(frame, area, chunk_slug, lines, scroll_offset, status, agent_list_state)` free fn with scrollable list and status line
- On `AgentDone` event: sets `AgentStatus` to `Done`/`Failed`, refreshes `self.milestones` and `self.cycle_slug`, calls `milestone_load` to refresh `self.detail_milestone` if `detail_run` applicable
- All 27 existing TUI integration tests pass without modification
- `cargo build -p assay-tui` and `cargo build -p assay-core` succeed

## Proof Level

- This slice proves: integration (real subprocess via `launch_agent_streaming`; synthetic event sequence via App integration tests)
- Real runtime required: no — echo subprocess proves streaming; real claude invocation is UAT only
- Human/UAT required: yes — pressing `r` from TUI dashboard with real InProgress project and claude installed is manual UAT

## Verification

- `cargo test -p assay-core --test pipeline` (or `-- launch_agent_streaming`) — asserts all echo lines received and exit code 0
- `cargo test -p assay-tui` — all 27 existing + new S01 tests pass (no regressions)
- `cargo build -p assay-tui` — produces `target/debug/assay-tui` without warnings
- New tests in `crates/assay-tui/tests/agent_run.rs`:
  - `agent_line_events_accumulate_in_agent_run_screen` — `App::handle_tui_event(AgentLine("foo"))` populates `Screen::AgentRun.lines`
  - `agent_done_event_transitions_to_done_status` — `App::handle_tui_event(AgentDone { exit_code: 0 })` sets `AgentStatus::Done { exit_code: 0 }`
  - `agent_done_nonzero_exit_sets_failed_status` — exit code 1 → `AgentStatus::Failed { exit_code: 1 }`
  - `r_key_no_active_chunk_is_noop` — App with no InProgress milestone: `r` key leaves screen as Dashboard
- New test in `crates/assay-core/tests/pipeline.rs` (or inline):
  - `launch_agent_streaming_delivers_all_lines` — real subprocess, asserts both lines received and join value == 0

## Observability / Diagnostics

- Runtime signals: `AgentStatus` inside `Screen::AgentRun` transitions `Running → Done/Failed`; exit code visible in status line; all agent stdout lines accumulated in `lines` Vec
- Inspection surfaces: TUI `Screen::AgentRun` status line shows "Running…", "Done (exit 0)", or "Failed (exit N)"; `draw_agent_run` renders all lines in a scrollable list
- Failure visibility: non-zero exit code surfaced as `AgentStatus::Failed { exit_code }` with rendered status line; `JoinHandle` panic (Err from join) mapped to exit code -1
- Redaction constraints: agent stdout lines passed through as-is; no credentials expected in stdout

## Integration Closure

- Upstream surfaces consumed: `assay-core::pipeline::HarnessWriter`, `assay-harness::claude::{generate_config, write_config, build_cli_args}`, `assay-core::milestone::cycle_status`, `assay-core::milestone::cycle::{CycleStatus}`, existing `App`/`Screen` in `app.rs`
- New wiring introduced in this slice: `launch_agent_streaming` in assay-core; `TuiEvent` enum + refactored `run()` in assay-tui; `r` key → `Screen::AgentRun` dispatch; `App::handle_tui_event` method for test-driving agent events
- What remains before the milestone is truly usable end-to-end: S02 (provider dispatch beyond hardcoded claude), S03 (slash command overlay), S04 (MCP panel)

## Tasks

- [x] **T01: Add `launch_agent_streaming` to assay-core pipeline** `est:45m`
  - Why: Provides the line-by-line subprocess streaming function; integration test proves real delivery before S02/S03 build on top
  - Files: `crates/assay-core/src/pipeline.rs`, `crates/assay-core/tests/pipeline.rs` (or new inline test)
  - Do: Add `pub fn launch_agent_streaming(cli_args: &[String], working_dir: &Path, line_tx: std::sync::mpsc::Sender<String>) -> std::thread::JoinHandle<i32>` — spawn child with `Stdio::piped()`, `BufReader::lines()` on stdout, send each line, return thread join handle carrying exit code (panic → -1). Leave existing `launch_agent()` fully untouched. Write integration test that spawns `sh -c 'printf "line1\nline2\n"; exit 0'` via `launch_agent_streaming`, collects all lines, joins thread, asserts `["line1", "line2"]` and exit code 0.
  - Verify: `cargo test -p assay-core` passes; `launch_agent_streaming_delivers_all_lines` test is green
  - Done when: `launch_agent_streaming` is pub in `assay-core::pipeline`, integration test green, no existing assay-core tests broken

- [ ] **T02: Add `TuiEvent`, `AgentStatus`, `Screen::AgentRun`, and `App` fields** `est:60m`
  - Why: Establishes the data model S01 tests will exercise; App fields must be initialized before the integration tests can construct App
  - Files: `crates/assay-tui/src/app.rs`, `crates/assay-tui/src/main.rs`, `crates/assay-tui/tests/agent_run.rs` (created failing)
  - Do: (1) Add `pub enum TuiEvent { Key(KeyEvent), Resize(u16, u16), AgentLine(String), AgentDone { exit_code: i32 } }` in `app.rs`. (2) Add `pub enum AgentStatus { Running, Done { exit_code: i32 }, Failed { exit_code: i32 } }`. (3) Add `Screen::AgentRun { chunk_slug: String, lines: Vec<String>, scroll_offset: usize, status: AgentStatus }` variant to `Screen` enum. (4) Add `pub agent_thread: Option<std::thread::JoinHandle<i32>>` and `pub agent_list_state: ListState` fields to `App`; initialize both to `None`/default in `with_project_root`. (5) Add `pub fn handle_tui_event(&mut self, event: TuiEvent) -> bool` method that handles `AgentLine` (pushes to lines if in `AgentRun`) and `AgentDone` (sets status, refreshes milestones/cycle_slug). (6) Create `crates/assay-tui/tests/agent_run.rs` with all four tests listed in Verification — they will compile after App fields are added but the `r`-key and event-handling logic is wired in T02/T03. Write them now so they drive T03 implementation. Add `draw_agent_run` stub (panics) to satisfy compilation.
  - Verify: `cargo test -p assay-tui` — all 27 existing tests pass; new `agent_run.rs` tests compile (some may fail until T03 implements the handlers)
  - Done when: `Screen::AgentRun`, `AgentStatus`, `TuiEvent`, `agent_thread`, `agent_list_state` all exist and compile; `agent_run.rs` test file committed

- [ ] **T03: Implement `handle_tui_event`, `r` key, `draw_agent_run`, and refactor `run()`** `est:90m`
  - Why: Wires everything together — event handling, key dispatch, rendering, and the channel-based main loop that makes the TUI non-blocking during agent execution
  - Files: `crates/assay-tui/src/app.rs`, `crates/assay-tui/src/main.rs`
  - Do: (1) Implement `App::handle_tui_event`: `AgentLine(s)` → if `Screen::AgentRun`, push to `lines`; `AgentDone { exit_code }` → set `AgentStatus::Done`/`Failed` based on code, call `self.agent_thread.take().map(|h| h.join())`, refresh `self.milestones` and `self.cycle_slug` from disk, return false. (2) Add `r` key to Dashboard arm: call `cycle_status(&assay_dir)`, get `active_chunk_slug`; if `None`, no-op; otherwise hardcode Anthropic adapter (`assay_harness::claude::generate_config` + `write_config` + `build_cli_args`), spawn `launch_agent_streaming` with a wrapper thread that converts lines to `TuiEvent::AgentLine` and sends `TuiEvent::AgentDone` — store `JoinHandle<i32>` in `self.agent_thread`, set `self.screen = Screen::AgentRun { chunk_slug, lines: vec![], scroll_offset: 0, status: AgentStatus::Running }`. For S01, the working_dir is `self.project_root` (simplified — full worktree setup is S02). (3) Implement `draw_agent_run(frame, area, chunk_slug, lines, scroll_offset, status, list_state: &mut ListState)`: scrollable `List` of line items, status line at bottom ("Running…" / "Done (exit 0)" / "Failed (exit N)"), `Esc` hint. (4) Wire `Screen::AgentRun` arm in `App::draw()`. (5) Add `Esc` handler in `Screen::AgentRun` arm of `handle_event` to return to Dashboard. (6) Refactor `main.rs::run()`: create `mpsc::channel::<TuiEvent>()`, spawn background thread converting crossterm events to `TuiEvent::Key`/`Resize`, main loop uses `rx.recv()` dispatching to `app.handle_event(key)` or `app.handle_tui_event(event)`. Note: the `r` key spawn must also push agent events through the same `tx` sender — pass `tx.clone()` to the agent wrapper thread. (7) Update `draw_agent_run` stub from T02 with real implementation.
  - Verify: `cargo test -p assay-tui` — all 27 existing + 4 new agent_run tests pass; `cargo build -p assay-tui` clean
  - Done when: all tests green including `agent_line_events_accumulate_in_agent_run_screen`, `agent_done_event_transitions_to_done_status`, `agent_done_nonzero_exit_sets_failed_status`, `r_key_no_active_chunk_is_noop`; `cargo build` clean

## Files Likely Touched

- `crates/assay-core/src/pipeline.rs`
- `crates/assay-tui/src/app.rs`
- `crates/assay-tui/src/main.rs`
- `crates/assay-tui/tests/agent_run.rs` (new)

## Observability / Diagnostics

- Failure path: `AgentStatus::Failed` renders "Failed (exit N)" in status line — immediately visible to developer
- Recovery: `Esc` from `AgentRun` always returns to Dashboard regardless of agent status
- Panic safety: `JoinHandle::join()` returning `Err` is mapped to exit code -1 (`AgentStatus::Failed { exit_code: -1 }`) — no panic propagation to TUI
