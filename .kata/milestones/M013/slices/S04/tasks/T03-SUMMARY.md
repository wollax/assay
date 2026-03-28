---
id: T03
parent: S04
milestone: M013
provides:
  - TUI wizard criteria step alternates name→cmd sub-steps via criteria_awaiting_cmd flag
  - assemble_inputs pairs name/cmd entries into CriterionInput structs with optional cmd
  - Prompt changes to "Command (Enter to skip):" during cmd sub-step
  - wizard_round_trip tests drive through cmd sub-steps and verify CriterionInput output
  - app_wizard test updated for cmd sub-step flow
key_files:
  - crates/assay-tui/src/wizard.rs
  - crates/assay-tui/tests/wizard_round_trip.rs
  - crates/assay-tui/tests/app_wizard.rs
key_decisions:
  - "criteria_awaiting_cmd bool flag operates within existing criteria step — no step index arithmetic changes (per S04-RESEARCH pitfall)"
  - "Fields in criteria step alternate name/cmd/name/cmd/trailing-empty — assemble_inputs iterates in pairs"
patterns_established:
  - "Sub-step flags (criteria_awaiting_cmd) for multi-phase input within a single wizard step index"
observability_surfaces:
  - "WizardState.criteria_awaiting_cmd is a public bool — inspectable in tests"
duration: 10min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T03: Update TUI wizard for cmd collection and verify all surfaces

**TUI wizard criteria step now collects optional command per criterion via name→cmd alternation, with all 1525 workspace tests passing**

## What Happened

Added `criteria_awaiting_cmd: bool` to `WizardState` to track whether the current input line is a criterion name or a command. When a criterion name is entered and Enter is pressed, the wizard switches to cmd sub-step (flag = true, new empty buffer for cmd input). After cmd is entered (or skipped with blank Enter), flag resets and a new buffer for the next criterion name is pushed. Blank Enter on a name line still terminates the criteria step.

Updated `assemble_inputs` to iterate the criteria fields in pairs (name at index j, cmd at index j+1), building `CriterionInput` structs with `cmd: None` for empty command strings.

Updated `step_prompt` to accept the `awaiting_cmd` flag and return "Command (Enter to skip):" when in cmd sub-step.

Updated all three test files that drive the wizard: `wizard_round_trip.rs` (updated driver + 2 new cmd-specific tests), `app_wizard.rs` (updated driver for cmd sub-step).

## Verification

- `cargo test -p assay-tui --test wizard_round_trip` — 11 tests pass including `test_wizard_criteria_cmd_round_trip` and `test_wizard_two_chunk_cmd_values`
- `cargo test -p assay-tui --test app_wizard` — 1 test passes (slug collision with cmd flow)
- `just ready` — 1525 tests pass, fmt/lint/deny all green

## Diagnostics

- `WizardState.criteria_awaiting_cmd` is a public bool — tests can assert on it during step-through
- Generated `gates.toml` files show `cmd` field presence/absence as the observable contract

## Deviations

- Also updated `crates/assay-tui/tests/app_wizard.rs` which wasn't listed in the task plan but used the old criteria flow and failed after the change. This was the expected cascading compile/test failure from changing the criteria step protocol.

## Known Issues

None

## Files Created/Modified

- `crates/assay-tui/src/wizard.rs` — Added `criteria_awaiting_cmd` flag, updated enter handler for name→cmd alternation, updated `assemble_inputs` to pair entries into `CriterionInput`, updated `step_prompt` for cmd prompt text
- `crates/assay-tui/tests/wizard_round_trip.rs` — Updated `drive_two_chunk_wizard` for cmd sub-steps, updated assertions for `CriterionInput`, added `test_wizard_criteria_cmd_round_trip` and `test_wizard_two_chunk_cmd_values`
- `crates/assay-tui/tests/app_wizard.rs` — Updated `drive_two_chunk_wizard` helper for cmd sub-steps
