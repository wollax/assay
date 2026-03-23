---
id: S03
parent: M007
milestone: M007
provides:
  - SlashCmd enum (GateCheck, Status, NextChunk, SpecShow, PrCreate)
  - parse_slash_cmd — case-insensitive command parsing from string input
  - execute_slash_cmd — synchronous dispatch to assay-core functions, returns String
  - SlashState struct with input/suggestion/result/error fields
  - SlashAction enum (Continue, Close, Execute) for event handler return
  - handle_slash_event — full keyboard dispatch (Esc/Enter/Tab/Char/Backspace)
  - draw_slash_overlay — bottom-aligned input line + result/error area
  - tab_complete — prefix-based completion against sorted COMMANDS array
  - App.slash_state field and `/` key handler from all non-wizard screens
  - 6 integration tests in slash_commands.rs
requires:
  - slice: S01
    provides: TuiEvent loop (receives `/` key from any screen)
affects: []
key_files:
  - crates/assay-tui/src/slash.rs
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/tests/slash_commands.rs
key_decisions:
  - "D111: Slash command dispatch is synchronous and in-process"
  - "COMMANDS array sorted alphabetically for deterministic tab completion"
  - "execute_slash_cmd returns error strings (never panics) for all failure paths"
patterns_established:
  - "Slash module as free functions (D001) — parse, dispatch, tab_complete, event handling"
  - "Error-as-string pattern for user-facing command output"
  - "Overlay state as Option<SlashState> on App, intercepted before screen dispatch (D104 pattern)"
observability_surfaces:
  - "SlashState.result captures successful command output"
  - "SlashState.error captures command failures"
  - "parse_slash_cmd and execute_slash_cmd are pure functions testable in isolation"
drill_down_paths:
  - .kata/milestones/M007/slices/S03/tasks/T01-SUMMARY.md
duration: 30min
verification_result: passed
completed_at: 2026-03-23
---

# S03: Slash Command Overlay

**`/` key opens a command overlay from any non-wizard TUI screen with tab completion; dispatches `/gate-check`, `/status`, `/next-chunk`, `/spec-show`, `/pr-create` to assay-core functions; proven by 6 integration tests**

## What Happened

Created `slash.rs` with the full slash command implementation in T01: `SlashCmd` enum, `parse_slash_cmd` (case-insensitive), `execute_slash_cmd` (synchronous dispatch to `cycle_status`, `load_spec_entry_with_diagnostics`, `pr_check_milestone_gates`, etc.), `tab_complete` (prefix match against sorted COMMANDS array), and `SlashState`/`SlashAction` types. T01 also created stubs for `handle_slash_event` and `draw_slash_overlay`.

T02 wired the overlay into the App: added `App.slash_state: Option<SlashState>` field, `/` key handler from Dashboard/MilestoneDetail/ChunkDetail/AgentRun/Settings screens, event interception before screen dispatch (consistent with D104 help overlay pattern), full `handle_slash_event` implementation (Esc closes, Enter dispatches, Tab completes, Char/Backspace edit input), and `draw_slash_overlay` rendering a bottom-aligned input line with result/error area.

## Verification

- `cargo test -p assay-tui --test slash_commands` — all 6 tests pass (parse_known_commands, parse_unknown_returns_none, tab_completes_partial_input, slash_key_opens_overlay, enter_dispatches_status_command, esc_closes_overlay)
- `cargo test -p assay-tui` — all 50 tests pass
- `just ready` — fmt, lint, test, deny all pass

## Requirements Advanced

- R056 (TUI slash commands) — `/` opens overlay, commands dispatch to assay-core, tab completion works

## Requirements Validated

- R056 — 6 integration tests prove parse, dispatch, tab completion, overlay open/close, and command execution

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- T02 was partially implemented during the same session as T01; the task boundary was blurred but all functionality is complete and tested.

## Known Limitations

- Slash commands are synchronous — long-running operations (e.g. gate evaluation on large specs) block the TUI until complete
- No command history or up-arrow recall
- `/pr-create` dispatches but depends on `gh` CLI being available and authenticated

## Follow-ups

- none

## Files Created/Modified

- `crates/assay-tui/src/slash.rs` — new module: SlashCmd, parse, dispatch, tab_complete, handle_slash_event, draw_slash_overlay
- `crates/assay-tui/src/lib.rs` — added `pub mod slash`
- `crates/assay-tui/src/app.rs` — App.slash_state field, `/` key handlers, event interception, draw dispatch
- `crates/assay-tui/tests/slash_commands.rs` — 6 integration tests

## Forward Intelligence

### What the next slice should know
- S03 is complete; no downstream slices in M007

### What's fragile
- Synchronous dispatch means gate evaluation blocks the TUI; for large specs this could freeze the UI for seconds

### Authoritative diagnostics
- `cargo test -p assay-tui --test slash_commands` — 6 tests cover parse/dispatch/overlay lifecycle

### What assumptions changed
- none
