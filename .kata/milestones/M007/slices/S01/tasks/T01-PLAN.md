---
estimated_steps: 7
estimated_files: 5
---

# T01: Define `TuiEvent`, `AgentRunStatus`, `Screen::AgentRun`, and `launch_agent_streaming` — with failing tests

**Slice:** S01 — Channel Event Loop and Agent Run Panel
**Milestone:** M007

## Description

This task establishes all new type contracts for the S01 slice and writes the integration test scaffold. No behavior is implemented yet for the App state machine methods (they are stubs). The `launch_agent_streaming` function in `assay-core::pipeline` IS fully implemented and tested here — it is a leaf function with no TUI dependencies. The test file is written in advance; most App state machine tests will compile but fail (or compile-error) until T02 adds real implementations.

Key constraints:
- `handle_event` signature (`bool` return, `KeyEvent` param) must NOT change — 27 tests depend on it.
- `TuiEvent` must be defined in `app.rs` (the library crate), not `main.rs`, so `App.event_tx` can reference it across the binary/lib boundary.
- Use `mpsc::channel()` (unbounded) for `launch_agent_streaming`'s `line_tx` to avoid deadlock.
- `Screen::AgentRun` requires a draw arm and handle_event arm to compile; both are stubs.

## Steps

1. **Add `TuiEvent` and `AgentRunStatus` enums to `app.rs`**
   - Add `use std::sync::mpsc;` import
   - Add `pub enum TuiEvent { Key(crossterm::event::KeyEvent), Resize(u16, u16), AgentLine(String), AgentDone { exit_code: i32 } }` above the `Screen` enum
   - Add `pub enum AgentRunStatus { Running, Done { exit_code: i32 }, Failed { exit_code: i32 } }` near `TuiEvent`

2. **Add `Screen::AgentRun` variant and stub dispatch**
   - Add variant `AgentRun { chunk_slug: String, lines: Vec<String>, scroll_offset: usize, status: AgentRunStatus }` to `Screen` enum
   - In `App::draw()` match arm, add: `Screen::AgentRun { chunk_slug, lines, scroll_offset, status } => { let cs = chunk_slug.clone(); let ls = lines.clone(); let so = *scroll_offset; let st = match status { AgentRunStatus::Running => "Running", AgentRunStatus::Done { .. } => "Done", AgentRunStatus::Failed { .. } => "Failed" }; draw_agent_run_stub(frame, content_area, &cs, &ls, so, st); }` — implement `fn draw_agent_run_stub(…)` as a minimal block with "AgentRun" title for now
   - In `App::handle_event()` match, add `Screen::AgentRun { .. } => false` arm (after Screen::Settings arm)

3. **Extend `App` struct with new fields**
   - Add `pub event_tx: Option<mpsc::Sender<TuiEvent>>` (initialized to `None` in `with_project_root`)
   - Add `pub agent_thread: Option<std::thread::JoinHandle<i32>>` (initialized to `None`)

4. **Add stub methods to `App`**
   - `pub fn handle_agent_line(&mut self, _line: String) {}` — intentional no-op stub
   - `pub fn handle_agent_done(&mut self, _exit_code: i32) {}` — intentional no-op stub

5. **Implement `launch_agent_streaming` in `assay-core::pipeline`**
   - Add `pub fn launch_agent_streaming(cli_args: &[String], working_dir: &std::path::Path, line_tx: std::sync::mpsc::Sender<String>) -> std::thread::JoinHandle<i32>` after `launch_agent()`
   - Inside: spawn a thread that (a) runs `Command::new(&cli_args[0]).args(&cli_args[1..]).current_dir(working_dir).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn()` — on spawn error, drop `line_tx` and return -1 immediately; (b) wraps stdout in `BufReader::lines()` and sends each line via `line_tx.send(line).ok()`; (c) after the BufReader is exhausted, waits for the process and returns the exit code (`child.wait().map(|s| s.code().unwrap_or(-1)).unwrap_or(-1)`)
   - The `JoinHandle` join value is the exit code `i32`

6. **Add `assay-harness` dep to `assay-tui/Cargo.toml`**
   - Add `assay-harness.workspace = true` under `[dependencies]`

7. **Write `crates/assay-tui/tests/agent_run.rs`**
   - Write all 8 tests described in S01-PLAN.md Verification section
   - Tests 1–2 (`launch_agent_streaming_*`) use a real `echo`/`sh -c 'exit 1'` subprocess — these should pass immediately after this task
   - Tests 3–8 call `App::handle_agent_line`, `App::handle_agent_done` — these compile but assert on stub no-ops, so they will FAIL until T02; mark with `#[allow(unused_variables)]` as needed and write the full assertions anyway (red-green discipline)

## Must-Haves

- [ ] `TuiEvent` and `AgentRunStatus` are `pub` in `assay_tui::app`
- [ ] `Screen::AgentRun` variant compiles (stub draw arm + stub handle_event arm present)
- [ ] `App.event_tx` and `App.agent_thread` fields exist and are `None` by default
- [ ] `App::handle_agent_line` and `App::handle_agent_done` are pub methods on `App` (stubs OK)
- [ ] `launch_agent_streaming` is pub in `assay_core::pipeline`, uses `BufReader::lines()`, returns `JoinHandle<i32>`
- [ ] `assay-harness` dep added to `assay-tui/Cargo.toml`
- [ ] `tests/agent_run.rs` written with all 8 tests
- [ ] `cargo check -p assay-tui` error-free
- [ ] `cargo test -p assay-core -- launch_agent_streaming` passes (tests 1–2)
- [ ] All 27 pre-existing `assay-tui` integration tests still pass

## Verification

```bash
# launch_agent_streaming tests pass
cargo test -p assay-core -- launch_agent_streaming

# All pre-existing TUI tests still pass (new agent_run tests may fail — expected)
cargo test -p assay-tui

# TUI compiles cleanly
cargo check -p assay-tui
```

## Observability Impact

- Signals added/changed: `TuiEvent` enum is the new runtime event type — future agents inspecting TUI behavior can reason about which events flow through the channel by examining the enum variants.
- How a future agent inspects this: `app.event_tx.is_some()` tells whether the channel is wired; `app.screen` tells the current state.
- Failure state exposed: `launch_agent_streaming` drops `line_tx` on spawn error, causing the relay-wrapper thread (T03) to see channel disconnect and report `AgentDone { exit_code: -1 }`.

## Inputs

- `crates/assay-tui/src/app.rs` — existing `Screen` enum, `App` struct, `handle_event()` — extend without breaking
- `crates/assay-core/src/pipeline.rs` — existing `launch_agent()` as reference pattern for `launch_agent_streaming`
- `crates/assay-core/src/orchestrate/gossip.rs` — reference mpsc + BufReader pattern for the streaming thread

## Expected Output

- `crates/assay-core/src/pipeline.rs` — `launch_agent_streaming()` added
- `crates/assay-tui/src/app.rs` — `TuiEvent`, `AgentRunStatus`, `Screen::AgentRun`, new App fields, stub methods
- `crates/assay-tui/Cargo.toml` — `assay-harness` dep added
- `crates/assay-tui/tests/agent_run.rs` — 8 integration tests (2 pass immediately, 6 fail/assert-fail until T02)
