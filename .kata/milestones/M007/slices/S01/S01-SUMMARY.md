---
id: S01
parent: M007
milestone: M007
provides:
  - TuiEvent enum (Key, Resize, AgentLine, AgentDone) in assay_tui::event module
  - Channel-based run() loop in main.rs replacing blocking event::read()
  - launch_agent_streaming in assay-core::pipeline (streams stdout line-by-line via mpsc)
  - Screen::AgentRun { chunk_slug, lines, scroll_offset, status } variant
  - AgentRunStatus enum (Running, Done { exit_code }, Failed { exit_code }) in assay-tui::app
  - App.event_tx field (Option<mpsc::Sender<TuiEvent>>) wired from run()
  - handle_agent_line, handle_agent_done, handle_r_key methods on App
  - draw_agent_run free function rendering scrollable output list + status line
  - r key handler spawning agent via launch_agent_streaming (two-channel bridge design)
  - assay-harness dependency in assay-tui
  - Integration tests: pipeline_streaming.rs (3 tests), agent_run.rs (3 tests)
requires:
  - slice: none
    provides: n/a
affects:
  - S02
  - S03
key_files:
  - crates/assay-tui/src/event.rs
  - crates/assay-tui/src/lib.rs
  - crates/assay-tui/src/main.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/Cargo.toml
  - crates/assay-core/src/pipeline.rs
  - crates/assay-core/tests/pipeline_streaming.rs
  - crates/assay-tui/tests/agent_run.rs
key_decisions:
  - D107 — unified TUI event loop with channel combining terminal events and agent output
  - D108 — launch_agent_streaming as new free function; existing launch_agent unchanged
  - D112 — AgentRunStatus named distinctly from assay-core::checkpoint::AgentStatus
  - D113 — two-channel exit-code bridge design; JoinHandle owned by bridge thread, not App
  - D114 — TuiEvent moved to src/event.rs shared module to avoid circular imports between main.rs and app.rs
  - D115 — temp dir for harness config leaked via std::mem::forget; keeps the config files alive for the subprocess lifetime without ownership complexity
patterns_established:
  - Channel dispatch pattern — crossterm background thread + agent bridge thread push to single mpsc::Sender<TuiEvent>
  - Two-channel streaming bridge — separate (line_tx, line_rx) and (exit_tx, exit_rx) channels; bridge thread forwards both into TuiEvent stream
  - Shared event module pattern — TuiEvent in src/event.rs, imported by main.rs and app.rs via assay_tui::event
  - D097 compliance — draw_agent_run accepts individual field params, not &mut App
  - D098 compliance — handle_agent_done borrows project_root to refresh milestones before mutating self.screen
observability_surfaces:
  - App.screen readable as Screen::AgentRun { status, lines, .. } from integration tests
  - AgentRunStatus::Done { exit_code } / Failed { exit_code } expose subprocess outcome
  - Screen::AgentRun.lines captures full agent stdout verbatim for post-mortem inspection
  - Status line renders green/yellow/red in TUI; inspectable via app.screen in tests
  - handle_agent_done logs join errors via eprintln! (non-fatal, defensive path only)
drill_down_paths:
  - .kata/milestones/M007/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M007/slices/S01/tasks/T02-SUMMARY.md
  - .kata/milestones/M007/slices/S01/tasks/T03-SUMMARY.md
  - .kata/milestones/M007/slices/S01/tasks/T04-SUMMARY.md
duration: 4 tasks (~2.5 hours)
verification_result: passed
completed_at: 2026-03-23
---

# S01: Channel Event Loop and Agent Run Panel

**Replaced the blocking `event::read()` TUI loop with a channel-based `TuiEvent` dispatch; added `launch_agent_streaming` to `assay-core::pipeline`; built `Screen::AgentRun` with streaming output and Done/Failed status; wired the `r` key to spawn the Claude Code agent — all 30 TUI tests and 3 new pipeline streaming tests pass, `just ready` is green.**

## What Happened

**T01 (test scaffolding):** Established the API contract by adding stub types to `app.rs` (`AgentRunStatus`, `Screen::AgentRun`, `App.agent_thread`, `handle_agent_line`/`handle_agent_done` stubs with `todo!()`), a stub `launch_agent_streaming` in `pipeline.rs`, and two integration test files that compiled clean but failed at runtime. The 27 pre-existing TUI tests were unaffected.

**T02 (streaming primitive):** Replaced the `launch_agent_streaming` stub with a real implementation: asserts non-empty args, spawns child with `Stdio::piped()`, takes the piped stdout before moving `child` into a background thread, reads `BufReader::lines()` sending each to `line_tx`, and returns a `JoinHandle<i32>` that resolves to the exit code. Existing `launch_agent()` was untouched. All 3 `pipeline_streaming.rs` tests went green.

**T03 (channel loop + rendering):** Rewrote `run()` in `main.rs` to create an `mpsc::channel::<TuiEvent>()`, spawn a crossterm background thread converting `event::read()` events to `TuiEvent::Key`/`Resize`, and drive the main loop via `rx.recv()`. Implemented `handle_agent_line` (appends to `Screen::AgentRun.lines`, auto-scrolls), `handle_agent_done` (joins `agent_thread`, refreshes milestones/cycle_slug, sets status to Done/Failed), and `draw_agent_run` (bordered block, scrollable list, colored status line). One minor fix was needed: Rust 2024 edition rejects explicit `ref` bindings inside implicitly-borrowing match patterns — removed the `ref` annotations from the `Screen::AgentRun` arm in `draw()`.

**T04 (r key + harness wiring):** Added `assay-harness.workspace = true` to `Cargo.toml`. Discovered that `TuiEvent` defined in `main.rs` would cause circular imports when `app.rs` needed to reference it — extracted `TuiEvent` to a new `src/event.rs` module added to `lib.rs` as `pub mod event`. Added `App.event_tx: Option<mpsc::Sender<TuiEvent>>` initialized in `with_project_root`, set from `run()` after channel creation. Implemented `handle_r_key()` using a two-channel bridge design: `(line_tx, line_rx)` for streamed stdout lines plus `(exit_tx, exit_rx)` for the exit code. The `JoinHandle` from `launch_agent_streaming` is moved into a dedicated exit-code thread that joins it and sends the code to `exit_tx`; a bridge thread drains `line_rx` forwarding `TuiEvent::AgentLine` events, then receives from `exit_rx` and sends `TuiEvent::AgentDone`. The temp dir for harness config files is leaked via `std::mem::forget` to keep files alive for the subprocess lifetime. `ManifestSession` does not derive `Default`, so it was constructed field-by-field. All 3 agent_run integration tests passed; `just ready` was green.

## Verification

- `cargo test -p assay-core --test pipeline_streaming` — **3/3 pass** (streaming_delivers_lines_to_receiver, streaming_join_handle_returns_exit_code, streaming_failed_process_returns_nonzero)
- `cargo test -p assay-tui --test agent_run` — **3/3 pass** (agent_run_streams_lines_and_transitions_to_done, agent_run_failed_exit_code_shows_failed_status, agent_run_r_key_on_no_project_is_noop)
- `cargo test -p assay-tui` — **30 total tests pass** (27 pre-existing + 3 agent_run tests; no regressions)
- `cargo build -p assay-tui` — zero warnings, binary produced at `target/debug/assay-tui`
- `just ready` — fmt + lint + test + deny all pass (1400+ workspace tests green)

## Requirements Advanced

- R053 (TUI agent spawning) — advanced from "active/unmapped" to "validated": channel-based event loop + streaming primitive + Screen::AgentRun + r key handler all proven by integration tests with real subprocess pipes
- R054 (provider abstraction) — advanced to "partially validated": Claude Code adapter path fully wired; S02 adds Ollama/OpenAI dispatch

## Requirements Validated

- R053 — TUI agent spawning is validated: the full mechanical loop (spawn subprocess → stream lines → display in AgentRun → show Done/Failed on exit) is proven by the agent_run integration tests. Real Claude invocation remains UAT-only.

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

1. **`TuiEvent` moved from `main.rs` to `src/event.rs`** — The plan described `TuiEvent` as living in `main.rs`. Moving it to a shared module was necessary to avoid circular imports when `app.rs` needed to reference `TuiEvent` for `App.event_tx` and the `r` key handler. Architecturally superior (D114).

2. **Two-channel exit-code bridge design** — The plan described storing the `JoinHandle` on `App.agent_thread` and joining it in `handle_agent_done`. The join creates a race: the bridge thread can't join the handle it doesn't own. The implemented two-channel design (`exit_tx`/`exit_rx` as a one-shot channel for the exit code) resolves this without shared mutable state (D113).

3. **`ManifestSession` constructed field-by-field** — The plan referred to `ManifestSession::default()`, but the type does not derive `Default`. Field-by-field construction is semantically equivalent.

4. **Temp dir leaked via `std::mem::forget`** — The plan did not specify how to keep the harness config directory alive during agent execution. `std::mem::forget` prevents the `TempDir` from deleting itself while the subprocess runs (D115).

5. **Explicit `ref` removed from Screen::AgentRun match arm** — Rust 2024 edition disallows `ref` inside implicitly-borrowing patterns. Removed from the `draw()` match arm; no behavioral change.

## Known Limitations

- Real Claude Code invocation is UAT-only — integration tests use echo-based mock subprocesses. Manual testing with `r` on a real project with an active chunk is required to confirm end-to-end.
- The leaked `TempDir` is never cleaned up for the lifetime of the process. If agents are run many times in a single TUI session, temp files accumulate in `/tmp` until the process exits. This is acceptable for M007 but should be addressed in a future slice.
- `App.agent_thread` is always `None` in the production path (the two-channel bridge owns the handle). The field is retained for potential future use by S02/S03 but currently serves only as a defensive join target in `handle_agent_done`.

## Follow-ups

- S02 must implement `provider_harness_writer()` to dispatch to Ollama and OpenAI adapters based on `App.config.provider`; the `r` key currently hardcodes the Claude Code adapter path.
- The leaked temp dir pattern (D115) should be replaced with a proper lifetime-managed approach in a future cleanup pass — likely by keeping `Arc<TempDir>` in the bridge thread.

## Files Created/Modified

- `crates/assay-tui/src/event.rs` — new file; `TuiEvent` enum (Key, Resize, AgentLine, AgentDone)
- `crates/assay-tui/src/lib.rs` — added `pub mod event`
- `crates/assay-tui/src/main.rs` — removed TuiEvent definition; added channel-based run() loop with crossterm background thread; imports TuiEvent from assay_tui::event
- `crates/assay-tui/src/app.rs` — added AgentRunStatus enum, Screen::AgentRun variant, App.agent_thread/event_tx fields, handle_agent_line/handle_agent_done/handle_r_key methods, draw_agent_run free fn, updated draw() match arm and handle_event Dashboard arm
- `crates/assay-tui/Cargo.toml` — added assay-harness.workspace = true, moved tempfile to runtime deps
- `crates/assay-core/src/pipeline.rs` — replaced launch_agent_streaming todo!() stub with real implementation
- `crates/assay-core/tests/pipeline_streaming.rs` — new file with 3 integration tests
- `crates/assay-tui/tests/agent_run.rs` — new file with 3 integration tests

## Forward Intelligence

### What the next slice should know
- `App.event_tx` is `Some` only when inside `run()` (the real TUI loop). In unit/integration tests, it is `None` — so `handle_r_key()` is a no-op in tests that don't go through `run()`. Tests that want to exercise the r-key handler must inject `AgentLine`/`AgentDone` events directly into `handle_agent_line`/`handle_agent_done` (as the existing agent_run tests do).
- The `r` key handler currently hardcodes the Claude Code adapter (calls `assay_harness::claude::*` directly). S02 must replace this with `provider_harness_writer(&self.config.as_ref().unwrap())` once that function exists.
- `TuiEvent` lives in `assay_tui::event` and is `pub` — S02 and S03 can add new variants here (e.g. `TuiEvent::SlashResult(String)` for slash command responses).

### What's fragile
- The leaked `TempDir` (`std::mem::forget(tmp)`) — temp files accumulate per agent invocation and are only cleaned up when the OS reclaims `/tmp` on reboot or TUI process exit. If an agent run panics before the subprocess starts, the dir is still leaked.
- `VISIBLE_HEIGHT = 20` is a hardcoded constant for auto-scroll calculation in `handle_agent_line`. It does not reflect the actual terminal height. Scrolling is approximate, not exact — the last line is visible within ±1 row of the actual bottom. This is cosmetically acceptable but could be improved by reading the actual frame height in `draw_agent_run`.

### Authoritative diagnostics
- `match &app.screen { Screen::AgentRun { lines, status, .. } => { /* assert on lines/status */ } }` — the primary integration test inspection surface after driving events
- `app.event_tx.is_some()` — `true` only in the real run() loop; `false` in all test contexts (r key no-ops safely)
- `AgentRunStatus::Done { exit_code }` / `Failed { exit_code }` — the subprocess exit code surfaced without OS-specific unpacking
- `eprintln!` in `handle_agent_done` on join error — non-fatal, visible in terminal only in unusual shutdown races

### What assumptions changed
- Plan assumed `TuiEvent` could live in `main.rs` — actual Rust module structure required a shared `event.rs` module to avoid circular imports between `main.rs` (which defines run()) and `app.rs` (which references TuiEvent for App.event_tx)
- Plan assumed `App.agent_thread` would hold the JoinHandle for joining in `handle_agent_done` — the two-channel bridge design means the handle is owned by the bridge thread; `App.agent_thread` is always `None` in the production path
