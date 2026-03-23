---
id: T01
parent: S03
milestone: M007
provides:
  - SlashCmd enum with 5 variants
  - parse_slash_cmd function (fully implemented)
  - execute_slash_cmd function (fully implemented, dispatches to assay-core)
  - SlashState struct with input/suggestion/result/error fields
  - SlashAction enum with Continue/Close/Execute variants
  - tab_complete function
  - handle_slash_event stub (returns Continue)
  - draw_slash_overlay stub (no-op)
  - 6 integration tests (3 green, 3 red-phase for T02)
key_files:
  - crates/assay-tui/src/slash.rs
  - crates/assay-tui/src/lib.rs
  - crates/assay-tui/tests/slash_commands.rs
key_decisions:
  - COMMANDS array sorted alphabetically for deterministic tab completion (first alpha match wins on ambiguity)
  - execute_slash_cmd uses cycle_status to find the active milestone before dispatching GateCheck/SpecShow/NextChunk/PrCreate, avoiding separate slug parameters
  - Red-phase tests use explicit panic!() with descriptive messages rather than compile-gated assertions, since App.slash_state field doesn't exist yet
patterns_established:
  - Slash module as free functions (D001 zero-trait convention) — parse, dispatch, tab_complete, stubs
  - Error-as-string pattern for execute_slash_cmd — never panics, returns user-readable error strings
observability_surfaces:
  - SlashState.result captures successful command output as String
  - SlashState.error captures command failures as String
  - parse_slash_cmd and execute_slash_cmd are pure functions testable in isolation
  - execute_slash_cmd returns error strings for: no active milestone, missing chunks, gate evaluation failures, spec load failures
duration: 10min
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T01: Create slash module with parse, dispatch, state types, and red-phase integration tests

**Added slash command module with full parse/dispatch implementation and 6 integration tests (3 green, 3 red-phase for T02).**

## What Happened

Created `crates/assay-tui/src/slash.rs` with all types and functions specified in the task plan:
- `SlashCmd` enum: GateCheck, Status, NextChunk, SpecShow, PrCreate
- `SlashState` struct: input, suggestion, result, error fields (Default derived)
- `SlashAction` enum: Continue, Close, Execute(SlashCmd)
- `COMMANDS` constant array (sorted alphabetically for deterministic tab completion)
- `parse_slash_cmd`: strips `/`, trims, case-insensitive match against COMMANDS
- `execute_slash_cmd`: real dispatch to `cycle_status`, `load_spec_entry_with_diagnostics`, `evaluate_all_gates` (via `pr_check_milestone_gates`), with error strings instead of panics
- `handle_slash_event`: stub returning `SlashAction::Continue`
- `draw_slash_overlay`: empty stub
- `tab_complete`: prefix match returning `/`-prefixed suggestion

Added `pub mod slash;` to lib.rs. Created 6 integration tests in `slash_commands.rs`.

## Verification

- `cargo build -p assay-tui` — zero warnings ✅
- `cargo test -p assay-tui --test slash_commands -- parse_known_commands` — passed ✅
- `cargo test -p assay-tui --test slash_commands -- parse_unknown_returns_none` — passed ✅
- `cargo test -p assay-tui --test slash_commands -- tab_completes_partial_input` — passed ✅
- `cargo test -p assay-tui --test slash_commands -- slash_key_opens_overlay` — failed (expected, red-phase for T02) ✅
- `cargo test -p assay-tui --test slash_commands -- enter_dispatches_status_command` — failed (expected, red-phase for T02) ✅
- `cargo test -p assay-tui --test slash_commands -- esc_closes_overlay` — failed (expected, red-phase for T02) ✅
- All existing assay-tui tests pass (help_status: 6, settings: 4, spec_browser: 6, wizard_round_trip: 9, app_wizard: 6) — zero regressions ✅

### Slice-level verification (partial — T01 is intermediate task):
- `cargo build -p assay-tui` — zero warnings ✅
- `cargo clippy -p assay-tui -- -D warnings` — pre-existing clippy warning in assay-types dependency (not from this change)

## Diagnostics

- `parse_slash_cmd` and `tab_complete` are pure functions — call directly in tests
- `execute_slash_cmd` returns user-readable strings for all outcomes including errors — inspect return value
- After T02 adds `App.slash_state`, inspect `app.slash_state.as_ref().unwrap().result` / `.error` for command outcomes

## Deviations

None.

## Known Issues

- Pre-existing clippy warning in `assay-types::RunManifest` (not from this task) causes `cargo clippy -p assay-tui -- -D warnings` to fail at the dependency level.

## Files Created/Modified

- `crates/assay-tui/src/slash.rs` — new file (~230 lines) with all slash command types, parse/dispatch/tab-complete functions, and stubs
- `crates/assay-tui/src/lib.rs` — added `pub mod slash;`
- `crates/assay-tui/tests/slash_commands.rs` — new file with 6 integration tests (3 pass, 3 red-phase)
