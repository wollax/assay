# S01: Channel Event Loop and Agent Run Panel

**Goal:** Refactor the TUI from a blocking `event::read()` loop to a channel-based `TuiEvent` loop; add `launch_agent_streaming` to `assay-core::pipeline`; build `Screen::AgentRun` with streaming output and Done/Failed status; wire the `r` key to spawn the agent with the Claude Code adapter.

**Demo:** An integration test in `crates/assay-tui/tests/agent_run.rs` drives a mock subprocess (spawned via `sh -c 'printf "line1\nline2\nline3\n"; exit 0'`), pumps `TuiEvent::AgentLine` and `TuiEvent::AgentDone` events through the real channel machinery, and asserts that `App.screen` is `Screen::AgentRun` with all three lines and `AgentRunStatus::Done { exit_code: 0 }`. A second test in `crates/assay-core/tests/pipeline_streaming.rs` proves `launch_agent_streaming` delivers lines to the receiver with a real subprocess and the join handle returns the correct exit code. All 27 existing TUI tests still pass. `cargo build -p assay-tui` compiles clean.

## Must-Haves

- `TuiEvent` enum defined in `main.rs`: `Key(crossterm::event::KeyEvent)`, `Resize(u16, u16)`, `AgentLine(String)`, `AgentDone { exit_code: i32 }`
- `run()` refactored to `mpsc::Receiver<TuiEvent>` loop; one background thread converts crossterm events; agent thread (when active) sends `AgentLine`/`AgentDone` to same sender
- `launch_agent_streaming(cli_args: &[String], working_dir: &Path, line_tx: mpsc::Sender<String>) -> std::thread::JoinHandle<i32>` in `assay-core::pipeline`; existing `launch_agent()` untouched
- `Screen::AgentRun { chunk_slug: String, lines: Vec<String>, scroll_offset: usize, status: AgentRunStatus }` variant in `Screen` enum
- `AgentRunStatus` enum: `Running`, `Done { exit_code: i32 }`, `Failed { exit_code: i32 }` — defined in `assay-tui::app` (not in `assay-core` to avoid name collision with `assay-core::checkpoint::AgentStatus`)
- `App.agent_thread: Option<std::thread::JoinHandle<i32>>` — joined for cleanup on `AgentDone`
- `r` key from `Screen::Dashboard` spawns agent when `cycle_status()` returns a non-None `active_chunk_slug`; transitions to `Screen::AgentRun`
- `draw_agent_run(frame, area, chunk_slug, lines, scroll_offset, status)` free fn — scrollable output list, status line at bottom
- On `TuiEvent::AgentDone`: refreshes `App.milestones`, `App.cycle_slug`, `App.detail_run`
- `assay-harness.workspace = true` added to `crates/assay-tui/Cargo.toml`
- Integration test `crates/assay-tui/tests/agent_run.rs` proves event loop delivers lines + Done transition
- Integration test `crates/assay-core/tests/pipeline_streaming.rs` proves `launch_agent_streaming` with real subprocess
- All 27 existing TUI tests still pass
- `cargo build -p assay-tui` compiles without warning
- `just ready` passes

## Proof Level

- This slice proves: integration — real subprocess pipes, real channel dispatch, real App state transitions
- Real runtime required: no (all tests use mock subprocesses; real Claude invocation is UAT)
- Human/UAT required: yes — pressing `r` on a real project and watching Claude output stream; verified manually after the slice is merged

## Verification

- `cargo test -p assay-core --test pipeline_streaming` — proves `launch_agent_streaming` line delivery and exit-code join
- `cargo test -p assay-tui` — all 27 existing tests still pass, plus new `agent_run` test suite
- `cargo test -p assay-tui --test agent_run` — specifically: `agent_run_streams_lines_and_transitions_to_done`, `agent_run_failed_exit_code_shows_failed_status`, `agent_run_no_active_chunk_r_key_is_noop`
- `cargo build -p assay-tui` — compiles with zero warnings
- `just ready` — fmt + lint + test + deny all pass

## Observability / Diagnostics

- Runtime signals: `AgentRunStatus` enum stored on `Screen::AgentRun` — a future agent can observe the exit code and status at any point after `AgentDone`; `lines: Vec<String>` captures full agent stdout for post-mortem
- Inspection surfaces: `App.screen` is public — integration tests (and future TUI diagnostics) read `Screen::AgentRun { status, lines, .. }` directly; exit code is in `AgentRunStatus::Done { exit_code }` / `Failed { exit_code }`
- Failure visibility: on `AgentDone`, status bar shows "Done (exit 0)" or "Failed (exit N)"; `App.agent_thread.take().map(|h| h.join())` performed for cleanup; join errors logged via `eprintln!` (non-fatal)
- Redaction constraints: agent stdout lines are streamed verbatim — no secrets redaction in M007 (raw subprocess output may contain API keys if user misconfigures Claude Code; future milestone concern)

## Integration Closure

- Upstream surfaces consumed: `assay-core::pipeline::HarnessWriter` type alias; `assay-core::milestone::cycle_status`; `assay-harness::claude::{generate_config, write_config, build_cli_args}`; `App.config: Option<assay_types::Config>` with `ProviderKind`
- New wiring introduced in this slice: channel-based `run()` loop in `main.rs`; `TuiEvent` enum; `Screen::AgentRun` + `AgentRunStatus` in `app.rs`; `launch_agent_streaming` in `assay-core::pipeline`; `r` key handler in `App::handle_event` Dashboard arm; `draw_agent_run` renderer
- What remains before the milestone is truly usable end-to-end: S02 (provider dispatch beyond Claude Code), S03 (slash command overlay), S04 (MCP panel) — all can be built on top of the S01 event loop

## Tasks

- [ ] **T01: Write failing integration tests for streaming and AgentRun** `est:45m`
  - Why: Test-first; establishes the exact API contract that T02–T04 must satisfy; tests fail until implementation lands, proving correctness
  - Files: `crates/assay-core/tests/pipeline_streaming.rs`, `crates/assay-tui/tests/agent_run.rs`
  - Do: Write `pipeline_streaming.rs` with tests for `launch_agent_streaming` (line delivery, exit code). Write `agent_run.rs` with tests that construct a real `TuiEvent` channel, drive `AgentLine`/`AgentDone` events into `App::handle_event_tui`, and assert `Screen::AgentRun` state. Tests must compile with placeholder/stub types added to `app.rs` (`AgentRunStatus` enum, `Screen::AgentRun` variant, `App::handle_event_tui` signature) but fail at runtime because implementations are absent. Do NOT implement the feature — only add the minimum type scaffolding needed to make tests compile.
  - Verify: `cargo test -p assay-core --test pipeline_streaming 2>&1 | grep "FAILED\|error\[E"` — expect compile errors or test failures (red). `cargo test -p assay-tui --test agent_run 2>&1 | grep "FAILED\|error\[E"` — same expectation.
  - Done when: Both test files exist, compile (perhaps with stub types), and the tests fail at runtime with "not yet implemented" or similar — but no compile errors

- [ ] **T02: Implement `launch_agent_streaming` in `assay-core::pipeline`** `est:45m`
  - Why: Provides the core streaming primitive that the TUI event loop depends on; proven independently before TUI wiring
  - Files: `crates/assay-core/src/pipeline.rs`
  - Do: Add `pub fn launch_agent_streaming(cli_args: &[String], working_dir: &Path, line_tx: mpsc::Sender<String>) -> std::thread::JoinHandle<i32>`. Spawn child with `Stdio::piped()`. In a new thread: read stdout via `BufReader::lines()`, send each line to `line_tx` (stop on `SendError`), wait for child exit, return exit code as the join value. Leave `launch_agent()` untouched. The `Sender<String>` parameter uses the existing `std::sync::mpsc` import already in the file.
  - Verify: `cargo test -p assay-core --test pipeline_streaming` — all tests in `pipeline_streaming.rs` pass
  - Done when: `cargo test -p assay-core --test pipeline_streaming` is green; `cargo test -p assay-core` still fully green (no regressions)

- [ ] **T03: Refactor `run()` to channel-based `TuiEvent` loop and add `Screen::AgentRun`** `est:1h`
  - Why: Core event loop refactor — replaces blocking `event::read()` with `mpsc::Receiver<TuiEvent>`; adds `Screen::AgentRun`, `AgentRunStatus`, `App.agent_thread`, and `draw_agent_run`
  - Files: `crates/assay-tui/src/main.rs`, `crates/assay-tui/src/app.rs`
  - Do: (1) In `main.rs`: define `TuiEvent` enum; refactor `run()` — create `mpsc::channel::<TuiEvent>()`, spawn crossterm background thread that loops `event::read()` and sends `TuiEvent::Key`/`TuiEvent::Resize`; main loop is `while let Ok(event) = rx.recv() { terminal.draw(...); match event { TuiEvent::Key(k) => app.handle_event(k), TuiEvent::Resize(..) => terminal.clear(), TuiEvent::AgentLine(l) => app.handle_agent_line(l), TuiEvent::AgentDone { exit_code } => app.handle_agent_done(exit_code), } }`. (2) In `app.rs`: add `AgentRunStatus` enum; add `Screen::AgentRun { chunk_slug, lines, scroll_offset, status }` variant; add `App.agent_thread: Option<std::thread::JoinHandle<i32>>`; add `handle_agent_line(&mut self, line: String)` method; add `handle_agent_done(&mut self, exit_code: i32)` method (joins thread, refreshes milestones/cycle_slug, updates status); add `draw_agent_run(frame, area, chunk_slug, lines, scroll_offset, status)` free fn; add `Screen::AgentRun { .. }` arm to `draw()`. Honor D097: `draw_agent_run` receives explicit fields. Honor D098: use `if let Screen::AgentRun { ref mut status, .. } = self.screen` only after scan completes. Auto-scroll: `scroll_offset = lines.len().saturating_sub(visible_height)`.
  - Verify: `cargo test -p assay-tui` — all 27 existing tests still pass (they use `handle_event()`, not `run()`, so they're unaffected by the loop refactor); count must be ≥27
  - Done when: `cargo test -p assay-tui` green with ≥27 tests; `cargo build -p assay-tui` compiles without warnings

- [ ] **T04: Wire `r` key handler and complete integration tests** `est:1h`
  - Why: Connects the `r` keypress to agent spawning via `launch_agent_streaming`; completes the `agent_run.rs` integration test suite; adds `assay-harness` dependency
  - Files: `crates/assay-tui/Cargo.toml`, `crates/assay-tui/src/app.rs`, `crates/assay-tui/tests/agent_run.rs`
  - Do: (1) Add `assay-harness.workspace = true` to `[dependencies]` in `crates/assay-tui/Cargo.toml`. (2) In `App::handle_event` Dashboard arm: on `KeyCode::Char('r')`, call `cycle_status(&assay_dir)`; if `Some(status)` with non-None `active_chunk_slug`, construct a `HarnessProfile` via `build_harness_profile(&ManifestSession { spec: chunk_slug.clone(), .. Default::default() })`, call `assay_harness::claude::generate_config(&profile)` then `assay_harness::claude::write_config(...)` to a temp dir, get `cli_args = assay_harness::claude::build_cli_args(&config)`; create `mpsc::channel::<String>()`, spawn `launch_agent_streaming(&cli_args, project_root, line_tx)`, store handle in `self.agent_thread`, create a new thread that converts `line_rx` lines to `TuiEvent::AgentLine` and on disconnect sends `TuiEvent::AgentDone { exit_code: handle.join() }`; transition to `Screen::AgentRun { chunk_slug, lines: vec![], scroll_offset: 0, status: AgentRunStatus::Running }`. NOTE: the agent thread sender needs to be captured from the `run()` function scope — the `r` handler must receive an `event_tx: mpsc::Sender<TuiEvent>` parameter. Refactor `App::handle_event` to accept `event_tx: Option<&mpsc::Sender<TuiEvent>>` (or store it on `App`). If no active chunk, `r` is a no-op. (3) Complete `agent_run.rs` tests: tests inject `TuiEvent::AgentLine`/`AgentDone` directly into `App::handle_agent_line`/`App::handle_agent_done` — no need to go through the full run loop for unit-level agent state tests. The test for `r` key dispatch uses a fake `event_tx` channel.
  - Verify: `cargo test -p assay-tui --test agent_run` — all three tests green; `cargo test -p assay-tui` — ≥27+3 tests green; `cargo build -p assay-tui` zero warnings; `just ready` passes
  - Done when: Full test suite green; `just ready` passes; `cargo build -p assay-tui` produces `target/debug/assay-tui` without warnings

## Files Likely Touched

- `crates/assay-tui/src/main.rs` — `TuiEvent` enum; refactored `run()` with channel loop and background crossterm thread
- `crates/assay-tui/src/app.rs` — `Screen::AgentRun`, `AgentRunStatus`, `App.agent_thread`, `handle_agent_line`, `handle_agent_done`, `draw_agent_run`, updated `draw()` match arm, updated `handle_event` with `event_tx` param
- `crates/assay-tui/Cargo.toml` — `assay-harness.workspace = true`
- `crates/assay-core/src/pipeline.rs` — `launch_agent_streaming` free function
- `crates/assay-core/tests/pipeline_streaming.rs` — new integration test file
- `crates/assay-tui/tests/agent_run.rs` — new integration test file
