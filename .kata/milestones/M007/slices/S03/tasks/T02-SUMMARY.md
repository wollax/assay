---
id: T02
parent: S03
milestone: M007
provides:
  - App.slash_state field wired into App lifecycle
  - Real handle_slash_event implementation (char input, Backspace, Tab, Enter, Esc)
  - Real draw_slash_overlay implementation (bottom-aligned input + result + hints)
  - Slash overlay guard in handle_event (D104 pattern — intercepts keys before screen dispatch)
  - "/" key handler in Dashboard, MilestoneDetail, ChunkDetail, Settings screens
  - execute_slash_cmd dispatch wired through App on Enter
  - All 6 integration tests now green
key_files:
  - crates/assay-tui/src/app.rs
  - crates/assay-tui/src/slash.rs
  - crates/assay-tui/tests/slash_commands.rs
key_decisions:
  - Slash overlay guard placed after help guard, before screen dispatch — slash takes priority over screen-specific keys when open
  - Execute result stored in SlashState.result while overlay stays open — user sees output and can Esc to close
  - Input buffer stores text without leading "/" — the "/" is rendered as a visual prefix in draw_slash_overlay
patterns_established:
  - Guard-before-dispatch pattern for overlays (help guard, then slash guard, then screen match)
  - Overlay result kept in state after dispatch — overlay stays open showing result until user presses Esc
observability_surfaces:
  - app.slash_state — Some(SlashState { result, error, .. }) when overlay is open; None when closed
  - SlashState.result — populated after successful command dispatch
  - SlashState.error — populated for unknown commands or missing project root
duration: 8min
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T02: Wire slash overlay into App event handling and drawing

**Wired slash command overlay into App with full event handling, drawing, and dispatch — all 6 integration tests green, zero regressions across 34 tests.**

## What Happened

Added `slash_state: Option<SlashState>` field to `App` struct, initialized to `None`. Implemented the real `handle_slash_event` function replacing the stub: handles character input (appending to buffer), Backspace (popping), Tab (filling suggestion into input), Enter (parsing and dispatching or showing error for unknown commands), and Esc (closing). Implemented the real `draw_slash_overlay` function: renders bottom-aligned within the content area with a cyan `/ ` prompt, current input, dimmed tab-completion suffix, result/error feedback line (green/red), and hint line.

Wired the overlay into `App::handle_event` with the D104 guard pattern: when `slash_state.is_some()`, all keys are intercepted before the `match self.screen` dispatch. On `SlashAction::Execute`, calls `execute_slash_cmd` with the project root and stores the result in `SlashState.result`. Added `/` key handler in Dashboard, MilestoneDetail, ChunkDetail, and Settings screen arms (not Wizard, NoProject, or LoadError per the plan).

Updated the three red-phase integration tests to use real assertions instead of `panic!()`. Fixed test fixture using wrong serde format (`"in-progress"` → `"in_progress"`).

## Verification

- `cargo test -p assay-tui --test slash_commands` — all 6 tests pass ✅
- `cargo test -p assay-tui` — all 34 tests pass, zero regressions ✅
- `cargo build -p assay-tui` — zero warnings ✅
- `cargo clippy -p assay-tui -- -D warnings` — only pre-existing assay-types warning (same as T01)

## Diagnostics

- Inspect `app.slash_state` — `Some(...)` means overlay is open, `None` means closed
- After dispatch: `app.slash_state.as_ref().unwrap().result` contains command output
- For errors: `app.slash_state.as_ref().unwrap().error` contains user-readable error string
- `handle_slash_event` is a pure function (given state + key → action) — testable in isolation

## Deviations

- Fixed test fixture serde format: `status = "in-progress"` → `status = "in_progress"` (T01 used wrong format, tests were red-phase so it didn't matter until now)

## Known Issues

- Pre-existing clippy warning in `assay-types::RunManifest` (derivable_impls) causes `cargo clippy -p assay-tui -- -D warnings` to fail at the dependency level. Not introduced by this task.

## Files Created/Modified

- `crates/assay-tui/src/slash.rs` — replaced `handle_slash_event` stub with real implementation (char/Backspace/Tab/Enter/Esc handling); replaced `draw_slash_overlay` stub with real bottom-aligned overlay renderer
- `crates/assay-tui/src/app.rs` — added `slash_state: Option<SlashState>` field; added slash overlay guard in `handle_event`; added `/` key handler in Dashboard/MilestoneDetail/ChunkDetail/Settings; added `draw_slash_overlay` call in `draw()`
- `crates/assay-tui/tests/slash_commands.rs` — replaced three `panic!()` red-phase tests with real assertions; fixed test fixture serde format
