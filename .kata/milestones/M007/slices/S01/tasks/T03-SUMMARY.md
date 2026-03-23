---
id: T03
parent: S01
milestone: M007
provides:
  - TuiEvent enum (pub) in main.rs with Key, Resize, AgentLine, AgentDone variants
  - Channel-based run() loop in main.rs replacing blocking event::read()
  - handle_agent_line implementation appending lines to Screen::AgentRun.lines with auto-scroll
  - handle_agent_done implementation joining agent_thread, refreshing milestones, updating status
  - draw_agent_run renderer with scrollable lines list and status line
  - Screen::AgentRun arm in App::draw() calling draw_agent_run
key_files:
  - crates/assay-tui/src/main.rs
  - crates/assay-tui/src/app.rs
key_decisions:
  - VISIBLE_HEIGHT = 20 constant used for auto-scroll offset calculation in handle_agent_line; exact precision is secondary to correctness (plan-specified)
  - explicit ref bindings removed from Screen::AgentRun match arm in draw() — Rust edition 2024 disallows explicit ref within implicitly-borrowing patterns
patterns_established:
  - Channel dispatch pattern — crossterm background thread sends TuiEvent::Key/Resize; agent thread sends TuiEvent::AgentLine/AgentDone; main loop uses rx.recv()
  - D097 compliance — draw_agent_run accepts individual field params (chunk_slug, lines, scroll_offset, status), not &mut App
  - D098 compliance — handle_agent_done borrows project_root to refresh milestones before mutating self.screen
observability_surfaces:
  - Screen::AgentRun.lines — full agent stdout captured verbatim for post-mortem inspection
  - Screen::AgentRun.status — transitions Running → Done { exit_code } / Failed { exit_code }; visible in status line (green/yellow/red)
  - handle_agent_done eprintln! on join error — non-fatal, surfaced in stderr
duration: short
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T03: Refactor `run()` to channel-based `TuiEvent` loop and implement `AgentRun` rendering

**Replaced the blocking `event::read()` loop with a channel-based `TuiEvent` dispatch, and implemented `handle_agent_line`, `handle_agent_done`, and `draw_agent_run` — all 3 agent_run integration tests now pass.**

## What Happened

`main.rs` was refactored from a blocking `event::read()` loop to a `mpsc::channel::<TuiEvent>()` dispatch loop. A background thread forwards crossterm `Event::Key` and `Event::Resize` as `TuiEvent::Key`/`TuiEvent::Resize`. The main loop calls `rx.recv()` and dispatches all four `TuiEvent` variants including `AgentLine` and `AgentDone`.

`handle_agent_line` in `app.rs` pushes the incoming line into `Screen::AgentRun.lines` and updates `scroll_offset` to keep the last lines visible (using `VISIBLE_HEIGHT = 20`).

`handle_agent_done` joins `self.agent_thread` (with `eprintln!` on join error), refreshes milestones and `cycle_slug` from disk (borrowing `project_root` before mutating `screen`, honoring D098), then sets `Screen::AgentRun.status` to `Done { exit_code }` or `Failed { exit_code }`.

`draw_agent_run` was added as a free function accepting individual field params (D097). It renders a bordered block titled "Agent Run: {chunk_slug}", a scrollable `List` of lines in the content area, and a 1-row status line colored yellow/green/red.

The `draw()` match arm for `Screen::AgentRun` was updated to call `draw_agent_run`.

One minor compiler fix was needed: the match arm used `ref chunk_slug, ref lines, ref status` which Rust 2024 edition rejects inside implicitly-borrowing patterns — removed the explicit `ref` annotations.

## Verification

```
cargo build -p assay-tui          → Finished (zero warnings, zero errors)
cargo test -p assay-tui --test agent_run
  → test agent_run_r_key_on_no_project_is_noop ... ok
  → test agent_run_failed_exit_code_shows_failed_status ... ok
  → test agent_run_streams_lines_and_transitions_to_done ... ok
  → test result: ok. 3 passed
cargo test -p assay-tui           → 30 total tests, all passed (27 pre-existing + 3 new agent_run)
```

## Diagnostics

- `match &app.screen { Screen::AgentRun { lines, status, .. } => { /* inspect */ } }` — read line buffer and status after driving events
- `AgentRunStatus::Done { exit_code }` / `Failed { exit_code }` — subprocess exit code surfaced in enum
- Status line renders green/yellow/red in TUI; inspectable via `app.screen` in tests
- `handle_agent_done` logs join errors to stderr (`eprintln!`) — non-fatal

## Deviations

- Explicit `ref` bindings removed from `Screen::AgentRun` match arm in `draw()` — Rust 2024 edition disallows `ref` inside implicitly-borrowing patterns (trivial fix, not a plan deviation in substance).

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/src/main.rs` — replaced blocking event::read() loop with channel-based TuiEvent dispatch; added TuiEvent enum; crossterm background thread
- `crates/assay-tui/src/app.rs` — implemented handle_agent_line, handle_agent_done, draw_agent_run; updated draw() Screen::AgentRun arm
