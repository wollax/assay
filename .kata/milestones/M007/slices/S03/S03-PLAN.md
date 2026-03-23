# S03: Slash Command Overlay

**Goal:** Pressing `/` from any non-wizard TUI screen opens a bottom-aligned command input overlay with tab completion; typing a command and pressing Enter dispatches synchronously to assay-core functions and displays the result inline; Esc closes the overlay — all proven by integration tests driving synthetic key events.

**Demo:** 6 integration tests in `crates/assay-tui/tests/slash_commands.rs` pass: parse correctness for all commands, unknown command returns None, `/` key opens overlay from Dashboard, Tab completes partial input, Enter dispatches `/status` and shows result, Esc closes overlay. `cargo test -p assay-tui` passes with zero regressions.

## Must-Haves

- `SlashCmd` enum with variants: `GateCheck`, `Status`, `NextChunk`, `SpecShow`, `PrCreate`
- `parse_slash_cmd(input: &str) -> Option<SlashCmd>` free function
- `execute_slash_cmd(cmd: SlashCmd, project_root: &Path) -> String` free function calling assay-core synchronously
- `SlashState { input: String, suggestion: Option<String>, result: Option<String>, error: Option<String> }` struct
- `SlashAction` enum: `Continue`, `Close`, `Execute(SlashCmd)`
- `handle_slash_event(state: &mut SlashState, key: KeyEvent) -> SlashAction` free function
- `draw_slash_overlay(frame, area: Rect, state: &SlashState)` free function — bottom-aligned input + result area
- `App.slash_state: Option<SlashState>` field (overlay on App, not a Screen variant)
- `/` key guard: opens overlay only from Dashboard, MilestoneDetail, ChunkDetail, AgentRun, Settings — NOT from Wizard, NoProject, LoadError
- Tab completion: prefix match against known command list; Tab fills suggestion into input
- Help overlay guard pattern (D104): when `slash_state.is_some()`, intercept all keys before `match self.screen`
- 6+ integration tests in `tests/slash_commands.rs` proving parse, dispatch, key handling, and overlay lifecycle
- `pub mod slash;` in `lib.rs`
- Zero-trait convention (D001): all free functions, no trait objects
- Synchronous dispatch (D111): no thread spawning for command execution

## Proof Level

- This slice proves: integration (synthetic key events → App state assertions; parse/dispatch against real assay-core functions on tempdir fixtures)
- Real runtime required: no (all tests use tempdir fixtures and synthetic events)
- Human/UAT required: yes — visual verification that the overlay renders correctly at the bottom of the terminal, that Tab completion feels responsive, and that `/gate-check` on a real project with runnable criteria shows meaningful results

## Verification

- `cargo test -p assay-tui --test slash_commands` — all 6+ tests pass
- `cargo test -p assay-tui` — all tests pass (existing 31+ plus new slash tests), zero regressions
- `cargo build -p assay-tui` — zero warnings
- `cargo clippy -p assay-tui -- -D warnings` — clean

## Observability / Diagnostics

- Runtime signals: `SlashState.error` field captures command execution failures as user-readable strings; `SlashState.result` captures successful output
- Inspection surfaces: `app.slash_state` field — `Some(SlashState { result: Some(...), .. })` indicates a command completed; integration tests inspect this directly
- Failure visibility: `execute_slash_cmd` returns error strings (not panics) for: no active chunk, no project root, gate evaluation failure, `gh` not found. All inspectable via `SlashState.error`
- Redaction constraints: none — slash commands operate on local file state only

## Integration Closure

- Upstream surfaces consumed: `TuiEvent` loop from S01 (receives `/` key as `TuiEvent::Key`); `App.project_root` for assay-core calls; `cycle_status` from `assay_core::milestone`; `evaluate_all_gates` from `assay_core::gate`; `load_spec_entry_with_diagnostics` from `assay_core::spec`; `pr_check_milestone_gates` from `assay_core::pr`
- New wiring introduced in this slice: `App.slash_state` guard in `handle_event` (before `match self.screen`); `/` key handler; `slash.rs` module with parse/dispatch/draw functions; `draw_slash_overlay` call in `App::draw` when `slash_state.is_some()`
- What remains before the milestone is truly usable end-to-end: S04 (MCP panel) is independent; S02 (provider dispatch) is independent. After all four slices: UAT on real project

## Tasks

- [ ] **T01: Create slash module with parse, dispatch, state types, and red-phase integration tests** `est:45m`
  - Why: Establishes the contract (types + parse + dispatch + tests) before wiring into App. Tests define the acceptance criteria and will initially fail for the overlay/App-wiring assertions.
  - Files: `crates/assay-tui/src/slash.rs`, `crates/assay-tui/src/lib.rs`, `crates/assay-tui/tests/slash_commands.rs`
  - Do: Create `slash.rs` with `SlashCmd` enum, `SlashState` struct, `SlashAction` enum, `parse_slash_cmd`, `execute_slash_cmd`, `handle_slash_event`, `draw_slash_overlay` stub. Export via `lib.rs`. Write 6 integration tests: `parse_known_commands`, `parse_unknown_returns_none`, `slash_key_opens_overlay`, `tab_completes_partial_input`, `enter_dispatches_status_command`, `esc_closes_overlay`. The parse tests pass immediately; overlay/App tests fail until T02.
  - Verify: `cargo test -p assay-tui --test slash_commands` — parse tests pass, overlay tests compile but may fail; `cargo build -p assay-tui` zero warnings
  - Done when: `slash.rs` has all 5 public types/fns, `lib.rs` exports `pub mod slash`, all 6 tests compile, parse tests pass

- [ ] **T02: Wire slash overlay into App event handling and drawing** `est:45m`
  - Why: Connects the slash module to the App lifecycle — the guard in `handle_event`, `/` key handler, `draw_slash_overlay` call in `draw()`, and Esc/Enter/Tab routing. This makes all T01 integration tests pass.
  - Files: `crates/assay-tui/src/app.rs`, `crates/assay-tui/src/slash.rs`
  - Do: Add `App.slash_state: Option<SlashState>` field. Add guard at top of `handle_event` (after help guard, before wizard check): when `slash_state.is_some()`, delegate to `handle_slash_event`, act on `SlashAction`. Add `/` key case in Dashboard/MilestoneDetail/ChunkDetail/AgentRun/Settings arms (or as a global match after the screen-specific match). Add `draw_slash_overlay` call in `draw()` when `slash_state.is_some()`. Implement real `handle_slash_event` (char input, Backspace, Tab completion, Enter dispatch, Esc close). Implement real `draw_slash_overlay` (bottom-aligned: input line + result area).
  - Verify: `cargo test -p assay-tui --test slash_commands` — all 6 tests pass; `cargo test -p assay-tui` — all tests pass with zero regressions; `cargo clippy -p assay-tui -- -D warnings` — clean
  - Done when: All 6 slash_commands tests green, all existing tests still green, `cargo build -p assay-tui` zero warnings

## Files Likely Touched

- `crates/assay-tui/src/slash.rs` — new module: SlashCmd, SlashState, SlashAction, parse_slash_cmd, execute_slash_cmd, handle_slash_event, draw_slash_overlay
- `crates/assay-tui/src/lib.rs` — add `pub mod slash;`
- `crates/assay-tui/src/app.rs` — add `slash_state` field to App, guard in handle_event, `/` key handler, draw_slash_overlay call in draw()
- `crates/assay-tui/tests/slash_commands.rs` — new integration test file with 6 tests
