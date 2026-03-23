---
estimated_steps: 5
estimated_files: 3
---

# T01: Create slash module with parse, dispatch, state types, and red-phase integration tests

**Slice:** S03 — Slash Command Overlay
**Milestone:** M007

## Description

Establish the complete slash command module (`slash.rs`) with all public types and functions, plus integration tests that define the acceptance criteria. The module contains `SlashCmd` enum, `SlashState` struct, `SlashAction` enum, `parse_slash_cmd`, `execute_slash_cmd`, `handle_slash_event` (stub returning `Continue`), and `draw_slash_overlay` (stub). Parse and dispatch functions are fully implemented; overlay-related functions are stubs. Integration tests for parse/dispatch pass immediately; overlay/App-wiring tests compile but fail until T02.

## Steps

1. Create `crates/assay-tui/src/slash.rs` with:
   - `SlashCmd` enum: `GateCheck`, `Status`, `NextChunk`, `SpecShow`, `PrCreate`
   - `SlashState` struct: `input: String`, `suggestion: Option<String>`, `result: Option<String>`, `error: Option<String>` — derive Default
   - `SlashAction` enum: `Continue`, `Close`, `Execute(SlashCmd)`
   - `COMMANDS: &[(&str, SlashCmd)]` constant array for tab completion and parsing
   - `parse_slash_cmd(input: &str) -> Option<SlashCmd>`: strip leading `/`, match against known commands (case-insensitive trim)
   - `execute_slash_cmd(cmd: SlashCmd, project_root: &Path) -> String`: real dispatch — `Status` calls `cycle_status`, `NextChunk` calls `cycle_status` + `load_spec_entry_with_diagnostics`, `GateCheck` calls `evaluate_all_gates`, `SpecShow` calls `load_spec_entry_with_diagnostics`, `PrCreate` calls `pr_check_milestone_gates`. All return formatted String results. Errors caught and returned as error strings (no panics).
   - `handle_slash_event(state: &mut SlashState, key: KeyEvent) -> SlashAction`: stub that always returns `SlashAction::Continue` (real impl in T02)
   - `draw_slash_overlay(frame: &mut Frame, area: Rect, state: &SlashState)`: empty stub (real impl in T02)
   - `tab_complete(input: &str) -> Option<String>`: prefix match against COMMANDS; returns full command name (with `/` prefix) if unique match, or first alphabetical match if multiple

2. Add `pub mod slash;` to `crates/assay-tui/src/lib.rs`

3. Create `crates/assay-tui/tests/slash_commands.rs` with helper functions (`key()`, `setup_project_with_milestone_and_chunks()`) and 6 tests:
   - `parse_known_commands` — all 5 commands parse correctly from `/gate-check`, `/status`, `/next-chunk`, `/spec-show`, `/pr-create`
   - `parse_unknown_returns_none` — `/foobar` returns `None`
   - `slash_key_opens_overlay` — create App with project, send `/` key, assert `app.slash_state.is_some()` (FAILS until T02 — app doesn't have the field yet)
   - `tab_completes_partial_input` — create SlashState with input `/sta`, call `tab_complete("sta")`, assert suggestion is `/status`
   - `enter_dispatches_status_command` — create App with InProgress milestone, open slash overlay, type `/status`, press Enter, assert `slash_state.result.is_some()` containing milestone info (FAILS until T02)
   - `esc_closes_overlay` — create App with slash overlay open, press Esc, assert `slash_state.is_none()` (FAILS until T02)

4. Ensure `cargo build -p assay-tui` compiles with zero warnings (stubs must suppress unused warnings with `_` prefixes on params or `#[allow(unused)]`)

5. Run `cargo test -p assay-tui --test slash_commands` — parse/tab_complete tests pass; overlay/App tests fail (expected red phase for T02)

## Must-Haves

- [ ] `SlashCmd` enum with 5 variants (GateCheck, Status, NextChunk, SpecShow, PrCreate)
- [ ] `parse_slash_cmd` correctly parses all 5 commands and returns None for unknown
- [ ] `execute_slash_cmd` dispatches to real assay-core functions (cycle_status, evaluate_all_gates, etc.)
- [ ] `SlashState` struct with input, suggestion, result, error fields
- [ ] `SlashAction` enum with Continue, Close, Execute variants
- [ ] `tab_complete` returns correct suggestion for prefix matches
- [ ] `pub mod slash;` in lib.rs
- [ ] 6 integration tests compile (some expected to fail until T02)
- [ ] Zero-trait convention (D001): all free functions

## Verification

- `cargo build -p assay-tui` — zero warnings, compiles clean
- `cargo test -p assay-tui --test slash_commands -- parse_known_commands` — passes
- `cargo test -p assay-tui --test slash_commands -- parse_unknown_returns_none` — passes
- `cargo test -p assay-tui --test slash_commands -- tab_completes` — passes
- `cargo test -p assay-tui` — existing 31+ tests still pass (no regressions)

## Observability Impact

- Signals added/changed: `SlashState.result` and `SlashState.error` capture command outcomes as strings — inspectable in tests
- How a future agent inspects this: `app.slash_state` field on App (after T02 adds it); `parse_slash_cmd` and `execute_slash_cmd` are pure functions testable in isolation
- Failure state exposed: `execute_slash_cmd` returns error strings for missing chunks, no active milestone, gate evaluation failures — never panics

## Inputs

- S03-RESEARCH.md — module structure, function signatures, assay-core function mapping
- S01-SUMMARY.md — TuiEvent enum, App field patterns, integration test helpers
- `crates/assay-core/src/milestone/cycle.rs` — `cycle_status` function signature and CycleStatus fields
- `crates/assay-core/src/gate/mod.rs` — `evaluate_all_gates` function signature
- `crates/assay-core/src/pr.rs` — `pr_check_milestone_gates` function signature
- `crates/assay-core/src/spec/mod.rs` — `load_spec_entry_with_diagnostics` function signature
- `crates/assay-tui/tests/agent_run.rs` — test helper patterns (setup_project_with_milestone, key())

## Expected Output

- `crates/assay-tui/src/slash.rs` — new file (~200 lines) with all types and functions
- `crates/assay-tui/src/lib.rs` — gains `pub mod slash;` line
- `crates/assay-tui/tests/slash_commands.rs` — new file with 6 integration tests
