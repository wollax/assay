---
id: T02
parent: S01
milestone: M007
provides:
  - "`pub enum TuiEvent` with variants Key, Resize, AgentLine, AgentDone"
  - "`pub enum AgentStatus` with variants Running, Done, Failed (all carry exit_code)"
  - "`Screen::AgentRun { chunk_slug, lines, scroll_offset, status }` variant"
  - "`App.agent_thread: Option<JoinHandle<i32>>` and `App.agent_list_state: ListState` fields initialized in with_project_root"
  - "`App::handle_tui_event` stub (returns false) — real dispatch in T03"
  - "`draw_agent_run` stub free function — real renderer in T03"
  - "`crates/assay-tui/tests/agent_run.rs` with 4 red-phase tests (3 fail until T03)"
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/tests/agent_run.rs
key_decisions:
  - "AgentStatus::Failed used for any non-zero exit code (including -1 from panicking thread); Done only for exit_code 0 — matches T01 JoinHandle<i32> contract"
  - "draw_agent_run is a free function (not a method) taking individual fields per D097; uses #[allow(unused_variables)] to keep build warning-free for the stub"
  - "handle_tui_event stub simply returns false — no match on event type — so tests compile without T03 implementation"
patterns_established:
  - "TuiEvent enum decouples terminal events (Key, Resize) from agent events (AgentLine, AgentDone) in a single receiver loop"
  - "Screen::AgentRun carries all rendering state inline (lines Vec, scroll_offset, status) — no external storage"
observability_surfaces:
  - "AgentStatus inside Screen::AgentRun is the runtime signal for agent execution state — inspect via `matches!(app.screen, Screen::AgentRun { status: AgentStatus::Done { .. }, .. })`"
  - "agent_thread field on App holds the JoinHandle — caller can join() to get exit code"
duration: 25min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T02: Add `TuiEvent`, `AgentStatus`, `Screen::AgentRun`, and `App` fields

**Added `TuiEvent`, `AgentStatus`, `Screen::AgentRun` enums plus `agent_thread`/`agent_list_state` App fields; stub `handle_tui_event` and `draw_agent_run`; created 4 failing integration tests that will pass after T03.**

## What Happened

Extended `crates/assay-tui/src/app.rs` with the complete data model for agent execution in the TUI:

- **`TuiEvent`** — public enum placed after imports; four variants: `Key(KeyEvent)`, `Resize(u16, u16)`, `AgentLine(String)`, `AgentDone { exit_code: i32 }`. Decouples terminal input from agent stdout events.
- **`AgentStatus`** — public enum adjacent to `TuiEvent`; three variants: `Running`, `Done { exit_code }`, `Failed { exit_code }`. Non-zero exit code always maps to `Failed`; `Done` is reserved for exit code 0.
- **`Screen::AgentRun`** — added to the `Screen` enum with `chunk_slug: String`, `lines: Vec<String>`, `scroll_offset: usize`, `status: AgentStatus`. Wired into all match exhaustion sites: `draw()` calls the `draw_agent_run` stub; `handle_event()` returns `false` (T03 adds Esc navigation).
- **`App` fields** — `agent_thread: Option<JoinHandle<i32>>` and `agent_list_state: ListState` added to struct and initialized in `with_project_root` (`None` and `ListState::default()` respectively).
- **`handle_tui_event` stub** — `pub fn handle_tui_event(&mut self, _event: TuiEvent) -> bool { false }`. Returns false unconditionally; T03 replaces with real dispatch.
- **`draw_agent_run` stub** — free function with `#[allow(unused_variables)]`, empty body, placed before `find_project_root`. Wired into `Screen::AgentRun` draw arm using `..` pattern for destructuring.
- **`crates/assay-tui/tests/agent_run.rs`** — created with 4 integration tests. `r_key_no_active_chunk_is_noop` passes immediately. The other three (`agent_line_events_accumulate_in_agent_run_screen`, `agent_done_event_transitions_to_done_status`, `agent_done_nonzero_exit_sets_failed_status`) fail as expected — they require T03's `handle_tui_event` implementation.

## Verification

- `cargo build -p assay-tui` — clean, no warnings
- `cargo test -p assay-tui --test app_wizard --test help_status --test settings --test spec_browser --test wizard_round_trip` — all 27 existing tests pass, zero regressions
- `cargo test -p assay-tui --test agent_run` — 4 tests compile; 1 passes (`r_key_no_active_chunk_is_noop`), 3 fail as expected (stub returns false without dispatching)
- `grep -n "agent_thread\|agent_list_state" crates/assay-tui/src/app.rs` — both fields visible at lines 131–133 (declaration) and 192–193 (initialization in with_project_root) and 252 (passed to draw_agent_run)

## Diagnostics

- Agent execution state observable via: `matches!(app.screen, Screen::AgentRun { status: AgentStatus::Done { exit_code }, .. })`
- Exit code preserved in both `Done` and `Failed` — accessible from `Screen::AgentRun.status` destructuring
- `App.agent_thread` is the join handle — `agent_thread.take().map(|h| h.join())` yields the exit code

## Deviations

None. Plan followed exactly. Steps 1–7 complete as specified.

## Known Issues

Three new tests are intentionally red until T03 implements `handle_tui_event`. This is by design per the task plan.

## Files Created/Modified

- `crates/assay-tui/src/app.rs` — added TuiEvent, AgentStatus enums; Screen::AgentRun variant; agent_thread/agent_list_state App fields; handle_tui_event stub; draw_agent_run stub; updated draw() and handle_event() match arms
- `crates/assay-tui/tests/agent_run.rs` — new file with 4 integration tests in red phase (1 passing, 3 failing until T03)
