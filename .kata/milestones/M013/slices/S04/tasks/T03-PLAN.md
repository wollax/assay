---
estimated_steps: 5
estimated_files: 2
---

# T03: Update TUI wizard for cmd collection and verify all surfaces

**Slice:** S04 — Wizard runnable criteria
**Milestone:** M013

## Description

Modify the TUI wizard's criteria step to collect an optional command after each criterion name, and run the full workspace verification to confirm all surfaces work together.

## Steps

1. In `crates/assay-tui/src/wizard.rs`: add a `criteria_awaiting_cmd: bool` field to `WizardState` (default `false`). This flag tracks whether the current input line is a cmd prompt (true) or a criterion name prompt (false). Initialize it in `WizardState::new()`.
2. Modify the criteria step in `handle_enter`: when `is_criteria` is true and `!criteria_awaiting_cmd`:
   - If current line is non-empty (criterion name entered): set `criteria_awaiting_cmd = true`, push a new empty string to `fields[step]` for cmd input, reset cursor. Do NOT advance step.
   - If current line is empty (blank enter = done with criteria): proceed as before (advance to next chunk or submit).
   When `is_criteria` is true and `criteria_awaiting_cmd`:
   - Store the cmd value (current line; empty → will become None). Set `criteria_awaiting_cmd = false`, push a new empty string to `fields[step]` for the next criterion name, reset cursor. Do NOT advance step.
3. Update `assemble_inputs` to pair up criteria entries: in the criteria step's field vec, entries alternate name/cmd/name/cmd/... (with trailing empty string from the blank-Enter terminator). Iterate in pairs: `name = fields[i]`, `cmd = fields[i+1]`; skip empty names; convert empty cmd to `None`. Build `CriterionInput { name, description: String::new(), cmd }`.
4. Update `step_prompt` and `draw_wizard`: when `criteria_awaiting_cmd` is true, show "Command (Enter to skip):" as the prompt instead of the criteria prompt.
5. Update `crates/assay-tui/tests/wizard_round_trip.rs`: modify `drive_two_chunk_wizard` to include cmd entries (type "cargo test" + Enter after criterion name, then blank Enter for cmd-skip on second). Add a new test asserting the assembled `WizardInputs` contains the correct `cmd` values. Run `just ready` to confirm all 1516+ workspace tests pass.

## Must-Haves

- [ ] `WizardState.criteria_awaiting_cmd` flag controls name-vs-cmd phase
- [ ] Criteria step alternates: name → cmd → name → cmd → blank (done)
- [ ] `assemble_inputs` pairs name/cmd entries into `CriterionInput` structs
- [ ] Prompt text changes to "Command (Enter to skip):" during cmd sub-step
- [ ] Updated wizard_round_trip test drives through cmd steps and verifies output
- [ ] `just ready` green with 1516+ tests

## Verification

- `cargo test -p assay-tui --test wizard_round_trip` — all tests pass including cmd-aware flow
- `just ready` — full workspace green, no regressions

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: `WizardState.criteria_awaiting_cmd` is a public bool — inspectable in tests
- Failure state exposed: None

## Inputs

- `crates/assay-core/src/wizard.rs` — T01's `CriterionInput` type and `WizardChunkInput` with `Vec<CriterionInput>`
- D094: ChunkCount uses replace semantics; criteria step follows existing event-handler patterns
- S04-RESEARCH.md pitfall: TUI state machine step index arithmetic is fragile — the cmd sub-step must NOT change the step count or step index arithmetic; it operates within the existing criteria step using the `criteria_awaiting_cmd` flag

## Expected Output

- `crates/assay-tui/src/wizard.rs` — `criteria_awaiting_cmd` field; updated enter handler; updated assembler; updated prompt
- `crates/assay-tui/tests/wizard_round_trip.rs` — updated driver helper; new cmd-aware test
