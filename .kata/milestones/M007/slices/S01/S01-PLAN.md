# S01: Channel Event Loop and Agent Run Panel

**Goal:** Refactor the TUI event loop from blocking `event::read()` to a channel-based `TuiEvent` dispatch loop, add `launch_agent_streaming()` to `assay-core::pipeline`, implement `Screen::AgentRun` with live line accumulation and Done/Failed status, and wire the `r` key in Dashboard to spawn the Claude agent and stream its output — all 27 existing TUI tests still pass.

**Demo:** Integration test in `crates/assay-tui/tests/agent_run.rs` drives a real echo subprocess through `launch_agent_streaming`, pumps resulting `AgentLine`/`AgentDone` events through `App::handle_agent_line` / `handle_agent_done`, and asserts `Screen::AgentRun` accumulates all lines and transitions to `AgentRunStatus::Done`. The channel event loop in `main.rs` is refactored to use `rx.recv()` over `Receiver<TuiEvent>` with a background crossterm thread. `r` key handler in Dashboard spawns the Claude harness relay-wrapper thread and transitions to `Screen::AgentRun`. All 27 pre-existing TUI integration tests still pass. `cargo build -p assay-tui` succeeds.

## Must-Haves

- `TuiEvent` enum in `assay_tui::app` with variants `Key(crossterm::event::KeyEvent)`, `Resize(u16, u16)`, `AgentLine(String)`, `AgentDone { exit_code: i32 }`
- `AgentRunStatus` enum in `assay_tui::app` with variants `Running`, `Done { exit_code: i32 }`, `Failed { exit_code: i32 }`
- `Screen::AgentRun { chunk_slug: String, lines: Vec<String>, scroll_offset: usize, status: AgentRunStatus }` variant on `Screen` enum
- `App.event_tx: Option<mpsc::Sender<TuiEvent>>` and `App.agent_thread: Option<std::thread::JoinHandle<i32>>` fields
- `App::handle_agent_line(&mut self, line: String)` — appends line to AgentRun.lines (cap 10 000); no-op on other screens
- `App::handle_agent_done(&mut self, exit_code: i32)` — sets AgentRunStatus, refreshes `milestones`, `cycle_slug`, `detail_run` from disk
- `draw_agent_run(frame, area, chunk_slug, lines, scroll_offset, status)` — scrollable output list, status bar, Esc hint; handles empty `lines` gracefully ("Starting…")
- `Screen::AgentRun` arm in `handle_event()` — Esc returns to Dashboard, `j`/`k`/`↑`/`↓` scroll lines
- `launch_agent_streaming(cli_args: &[String], working_dir: &Path, line_tx: mpsc::Sender<String>) -> std::thread::JoinHandle<i32>` in `assay_core::pipeline` — spawns subprocess, reads stdout line-by-line via `BufReader::lines()`, sends each line to `line_tx`; returns join handle; exit code delivered via `JoinHandle::join()`
- `assay-harness` added as workspace dep to `crates/assay-tui/Cargo.toml`
- `run()` in `main.rs` refactored to `mpsc::Receiver<TuiEvent>` loop; background crossterm thread; `app.event_tx` set before the loop
- `r` key handler in Dashboard arm: gets active chunk via `cycle_status`, builds `HarnessProfile`, calls `write_config` + `build_cli_args`, spawns relay-wrapper thread → `Screen::AgentRun`; no-op when `event_tx` is `None`
- All 27 pre-existing `assay-tui` integration tests pass without modification
- `cargo test -p assay-tui` all green; `cargo build -p assay-tui` succeeds

## Proof Level

- This slice proves: **integration** — `launch_agent_streaming` proven with a real echo subprocess; App state machine proven by direct method calls; existing tests prove no regression from event loop refactor
- Real runtime required: no — echo subprocess replaces real Claude for test proof; real Claude invocation is UAT-only
- Human/UAT required: yes — for the actual `r` key → Claude streaming experience in a real project

## Verification

All commands must pass before the slice is considered done:

```bash
# All new integration tests pass
cargo test -p assay-tui --test agent_run

# All 27 pre-existing TUI tests still pass
cargo test -p assay-tui

# launch_agent_streaming unit test in pipeline
cargo test -p assay-core -- launch_agent_streaming

# Build succeeds (no deadlock, no panic at compile time)
cargo build -p assay-tui

# Full workspace check
just ready
```

Test file: `crates/assay-tui/tests/agent_run.rs`

Tests in scope:
1. `launch_agent_streaming_delivers_all_lines` — real echo subprocess; all N lines arrive via `line_tx`
2. `launch_agent_streaming_delivers_exit_code` — zero exit for `true`, nonzero for `false`
3. `handle_agent_line_accumulates_in_agent_run_screen` — pump lines through App, assert `Screen::AgentRun.lines`
4. `handle_agent_done_zero_exit_transitions_to_done` — `AgentRunStatus::Done { exit_code: 0 }`
5. `handle_agent_done_nonzero_exit_transitions_to_failed` — `AgentRunStatus::Failed { exit_code: 1 }`
6. `handle_agent_line_caps_at_ten_thousand` — insert 10 001 lines; assert len == 10 000
7. `handle_agent_line_noops_on_non_agent_run_screen` — pumping lines on Dashboard is a no-op (no panic)
8. `r_key_noops_when_event_tx_is_none` — `r` on Dashboard with `event_tx = None` doesn't panic, screen unchanged

## Observability / Diagnostics

- **Runtime signals:** `AgentRunStatus` is visible in `Screen::AgentRun.status` — future agents can inspect `app.screen` to determine whether the agent is `Running`, `Done`, or `Failed`. `lines` contains the full raw subprocess output (capped at 10 000) for post-mortem diagnosis.
- **Inspection surfaces:** After exit, `Screen::AgentRun.lines` holds the complete stdout; `status` holds the final exit code. The TUI renders "Done (exit 0)" or "Failed (exit N)" in the status line.
- **Failure visibility:** Non-zero exit code is preserved in `AgentRunStatus::Failed { exit_code }` and rendered to the screen; `r` key handler displays an inline error on `Screen::AgentRun` (or the Dashboard error line) when `cycle_status` returns `None` (no active chunk) or when `write_config` fails.
- **Deadlock prevention:** `launch_agent_streaming` uses `mpsc::channel()` (unbounded), not `sync_channel`. The relay-wrapper thread serializes: drain `str_rx` → join inner `JoinHandle<i32>` → send `TuiEvent::AgentDone`. This guarantees no line is lost before `AgentDone` is emitted.
- **Redaction constraints:** subprocess stdout may contain API keys or model output; do not log these outside of the `lines` buffer.

## Integration Closure

- **Upstream consumed:** `assay_core::pipeline::launch_agent_streaming` (new); `assay_harness::claude::{generate_config, write_config, build_cli_args}` (existing); `assay_core::milestone::cycle_status` (existing); `App::with_project_root`, `App::handle_event`, existing 27 tests (all unchanged).
- **New wiring introduced:** `main.rs::run()` ↔ `mpsc::channel::<TuiEvent>()` ↔ crossterm background thread; `r` key handler → harness config write → `launch_agent_streaming` relay-wrapper thread → `TuiEvent::AgentLine`/`AgentDone` pushed to same channel.
- **Remaining before milestone end-to-end:** S02 adds provider dispatch (Ollama, OpenAI alternatives); S03 adds slash command overlay; S04 adds MCP server panel. Real Claude invocation is UAT-only until S02 confirms provider wiring.

## Tasks

- [x] **T01: Define `TuiEvent`, `AgentRunStatus`, `Screen::AgentRun`, and `launch_agent_streaming` — with failing tests** `est:1h`
  - Why: Establishes all type contracts and integration test scaffold before any behavior is implemented; tests fail until T02 makes them pass; existing 27 tests confirm no compile regressions.
  - Files: `crates/assay-core/src/pipeline.rs`, `crates/assay-tui/src/app.rs`, `crates/assay-tui/Cargo.toml`, `crates/assay-tui/tests/agent_run.rs`
  - Do: (1) Add `pub enum TuiEvent` and `pub enum AgentRunStatus` to `app.rs`; (2) add `Screen::AgentRun { chunk_slug, lines, scroll_offset, status }` variant to `Screen` — add a stub arm to `draw()` (`draw_agent_run` stub that renders a placeholder block) and to `handle_event()` (returns `false`); (3) add `event_tx: Option<mpsc::Sender<TuiEvent>>` and `agent_thread: Option<std::thread::JoinHandle<i32>>` fields to `App` (initialized to `None`); (4) add pub stub methods `handle_agent_line(&mut self, line: String)` (no-op body for now) and `handle_agent_done(&mut self, exit_code: i32)` (no-op body) to `App`; (5) add `pub fn launch_agent_streaming(cli_args: &[String], working_dir: &Path, line_tx: std::sync::mpsc::Sender<String>) -> std::thread::JoinHandle<i32>` to `pipeline.rs` using `Command::new(&cli_args[0]).args(&cli_args[1..]).current_dir(working_dir).stdout(Stdio::piped()).spawn()`, then BufReader::lines() in the spawned thread to send each line, return join handle with exit code; (6) add `assay-harness = { workspace = true }` to `[dependencies]` in `assay-tui/Cargo.toml`; (7) write `crates/assay-tui/tests/agent_run.rs` with all 8 tests listed in the Verification section — tests for the App state machine (T03–T08) will fail to compile or fail to assert until T02's implementations are in place, but `launch_agent_streaming` tests (T01–T02) should pass immediately.
  - Verify: `cargo test -p assay-core -- launch_agent_streaming` passes; `cargo check -p assay-tui` compiles cleanly; `cargo test -p assay-tui` — all 27 pre-existing tests still pass (new agent_run tests may fail until T02).
  - Done when: `cargo check -p assay-tui` is error-free; `cargo test -p assay-core -- launch_agent_streaming` green; 27 pre-existing TUI tests still pass.

- [x] **T02: Implement `handle_agent_line`, `handle_agent_done`, and `draw_agent_run`** `est:1h`
  - Why: Closes the App state machine loop — events flowing into App produce the correct `Screen::AgentRun` state transitions, and the renderer correctly displays the accumulated output; makes the agent_run integration tests pass.
  - Files: `crates/assay-tui/src/app.rs`
  - Do: (1) Implement `handle_agent_line`: if `self.screen` is `Screen::AgentRun { lines, .. }`, push `line` to `lines`; if `lines.len() > 10_000`, remove the first element (or truncate); no-op on all other screens; (2) implement `handle_agent_done`: if `self.screen` is `Screen::AgentRun { status, .. }`, set `status = if exit_code == 0 { AgentRunStatus::Done { exit_code } } else { AgentRunStatus::Failed { exit_code } }`; then refresh `self.milestones` and `self.cycle_slug` from disk via `milestone_scan` + `cycle_status` (using `self.project_root` guard); if `self.detail_run` is Some, optionally refresh — keep it simple for now; (3) implement `draw_agent_run(frame, area, chunk_slug, lines, scroll_offset, status)` free function: render a bordered Block titled "Agent Run: {chunk_slug}", show the last N lines that fit in the area minus 2 (for status line), if `lines.is_empty()` show "Starting…" item, bottom line shows "Running…" / "Done (exit 0)" / "Failed (exit N)" styled appropriately, bottom-right hint "Esc: back"; (4) replace the stub arm in `draw()` to call real `draw_agent_run(…)`; (5) implement `Screen::AgentRun` arm in `handle_event()`: `Esc` → `self.screen = Screen::Dashboard`; `KeyCode::Down` / `Char('j')` → scroll_offset += 1 (capped at lines.len()); `KeyCode::Up` / `Char('k')` → scroll_offset = scroll_offset.saturating_sub(1); other keys → no-op.
  - Verify: `cargo test -p assay-tui --test agent_run` — all 8 tests in agent_run.rs pass; `cargo test -p assay-tui` — all pre-existing 27 tests still pass.
  - Done when: All 8 agent_run integration tests are green; 27 pre-existing tests unchanged.

- [x] **T03: Refactor `run()` to channel-based event loop + wire `r` key handler** `est:1.5h`
  - Why: Makes the TUI actually stream agent output — the channel loop is the runtime backbone that delivers `AgentLine`/`AgentDone` events to the App; the `r` key handler is the user-facing entry point that wires harness config → subprocess → channel.
  - Files: `crates/assay-tui/src/main.rs`, `crates/assay-tui/src/app.rs`
  - Do: (1) In `main.rs`, replace `event::read()` loop with: `let (tx, rx) = std::sync::mpsc::channel::<TuiEvent>();` (unbounded); spawn a background thread that loops `crossterm::event::read()` and sends `TuiEvent::Key(k)` or `TuiEvent::Resize(w, h)` to `tx`; call `app.event_tx = Some(tx.clone())`; change the main loop to `while let Ok(event) = rx.recv()` dispatching `TuiEvent::Key(k)` → `app.handle_event(k)`, `TuiEvent::Resize(..)` → `terminal.clear()`, `TuiEvent::AgentLine(line)` → `app.handle_agent_line(line)`, `TuiEvent::AgentDone { exit_code }` → `app.handle_agent_done(exit_code)`; (2) In `app.rs` Dashboard arm of `handle_event()`, add `KeyCode::Char('r')` handler: guard `if let Some(tx) = self.event_tx.clone()` — if `None`, no-op; get `assay_dir` from `project_root`; call `cycle_status(&assay_dir)` — if `None` or error, set a transient screen error (or just no-op for S01 simplicity); get `chunk_slug`; construct a minimal `HarnessProfile` (use the same `HarnessProfile::default()` approach already used in existing harness tests or pipeline tests); call `assay_harness::claude::generate_config(&profile)`, then `write_config(&claude_config, &assay_dir.parent().unwrap())` for the temp worktree dir — for S01, write to `assay_dir` itself (the r-key is a proof-of-concept; S02 will add full worktree setup); call `build_cli_args(&claude_config)` to get `cli_args`; spawn the relay-wrapper thread: inside it, create `(str_tx, str_rx) = mpsc::channel::<String>()`, spawn `launch_agent_streaming(cli_args, working_dir, str_tx)` as the inner thread returning `JoinHandle<i32>`, drain `str_rx` sending each `TuiEvent::AgentLine(line)` to `tx`, after drain join inner handle to get `exit_code`, send `TuiEvent::AgentDone { exit_code }` to `tx`; store `JoinHandle` of the wrapper in `self.agent_thread`; transition `self.screen` to `Screen::AgentRun { chunk_slug, lines: vec![], scroll_offset: 0, status: AgentRunStatus::Running }`.
  - Verify: `cargo build -p assay-tui` succeeds; `cargo test -p assay-tui` — all tests pass (tests bypass `run()` and still work since `event_tx = None` is safe); `cargo clippy -p assay-tui` clean.
  - Done when: `cargo build -p assay-tui` produces binary without error; all existing 27 tests + 8 new agent_run tests pass.

- [ ] **T04: Final cleanup and `just ready` green** `est:30m`
  - Why: Ensures the slice meets milestone quality gates (fmt, clippy, cargo-deny, all tests) before moving to S02 which depends on S01's channel loop being stable.
  - Files: `crates/assay-tui/src/app.rs`, `crates/assay-tui/src/main.rs`, `crates/assay-core/src/pipeline.rs` (minor fixups only)
  - Do: (1) Run `cargo fmt --all` and fix any formatting issues introduced in T01–T03; (2) run `cargo clippy --workspace --all-targets` and fix any lints (unused imports, dead_code, missing pub, etc.); (3) run `cargo test --workspace` and confirm all tests pass; (4) run `cargo deny check` and confirm no new advisories; (5) verify `crates/assay-tui/Cargo.toml` has the `[[bin]] name = "assay-tui"` section (added in M006/D088 — confirm it's present, add if missing); (6) add `#[allow(dead_code)]` or fix any unused items flagged by clippy in the new code.
  - Verify: `just ready` exits 0; `cargo build -p assay-tui` produces `target/debug/assay-tui`.
  - Done when: `just ready` is fully green with no warnings or errors.

## Files Likely Touched

- `crates/assay-core/src/pipeline.rs` — new `launch_agent_streaming()` function
- `crates/assay-tui/src/app.rs` — `TuiEvent`, `AgentRunStatus`, `Screen::AgentRun`, new `App` fields, `handle_agent_line`, `handle_agent_done`, `draw_agent_run`, event handler arms
- `crates/assay-tui/src/main.rs` — channel-based `run()` refactor
- `crates/assay-tui/Cargo.toml` — add `assay-harness` dep
- `crates/assay-tui/tests/agent_run.rs` — new integration test file (8 tests)
