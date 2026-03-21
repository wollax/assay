---
id: T02
parent: S02
milestone: M006
provides:
  - WizardState::new() — 3 initial field slots (name, description, chunk-count), zeroed counters
  - WizardState::current_step_kind() — pure index→StepKind mapping for any N in 1..7
  - handle_wizard_event() — full pure state machine: char input, backspace nav, Enter validation/allocation/submit, Esc cancel
  - Dynamic field allocation — after ChunkCount confirmed, exactly N ChunkName + N Criteria vecs pushed
  - assemble_submit() helper — builds WizardInputs with slugified names, filtered criteria
  - 13 unit tests covering step routing, backspace nav, criteria entry, submit assembly, validation, non-press guard
key_files:
  - crates/assay-tui/src/wizard.rs
key_decisions:
  - ChunkCount only accepts '1'–'7'; '0', '8'+, and non-digits are silently ignored (field replace, not append)
  - Default impl delegates to new() so WizardState is usable in ratatui App structs without explicit construction
  - assemble_submit() is a free function (not method) to avoid borrow issues when passing &WizardState to it after consuming state.step/chunk_count
patterns_established:
  - State machine pattern: all event routing branches on StepKind enum from current_step_kind(), never raw step index
  - Criteria multi-line pattern: Vec<String> per criteria step; blank Enter advances/submits; backspace pops entry or goes back
observability_surfaces:
  - state.error: Option<String> — set on empty name, invalid chunk count; cleared on next press; visible inline in wizard UI
  - cargo test -p assay-tui -- --nocapture — full test trace including integration test file paths in tempdir
duration: 35min
verification_result: passed
completed_at: 2026-03-20T00:00:00Z
blocker_discovered: false
---

# T02: Implement WizardState state machine and make integration test green

**Pure WizardState state machine fully implemented — 13 unit tests + wizard_round_trip integration test all green.**

## What Happened

Replaced all `todo!("T02")` stubs in `crates/assay-tui/src/wizard.rs` with the complete state machine:

- `WizardState::new()` initializes with three field vecs (name/description/chunk-count slots), all counters at zero.
- `current_step_kind()` maps raw step indices to `StepKind` variants using chunk_count: steps 0–2 are fixed, `3..3+N` are `ChunkName(i)`, `3+N..3+2N` are `Criteria(i)`.
- `handle_wizard_event()` guards `KeyEventKind::Press` first, clears `state.error` on every press, then dispatches on `KeyCode`:
  - `Char` for ChunkCount: replace-semantics, only '1'–'7' accepted
  - `Char` for other steps: append to active buffer, increment cursor
  - `Backspace` for single-line steps: pop char or go back a step
  - `Backspace` for Criteria: pop char, or remove trailing empty entry, or go back a step
  - `Enter` for Name: validate non-empty, advance
  - `Enter` for ChunkCount: validate digit 1–7, set chunk_count, push N+N vecs, advance
  - `Enter` for ChunkName: advance
  - `Enter` for Criteria: push new empty entry (if non-blank), or advance/submit (if blank)
  - `Esc`: return Cancel immediately
- `assemble_submit()` builds WizardInputs with `slugify()`, filters empty criterion strings.
- Added `Default` impl (delegates to `new()`) for ergonomic use in ratatui App structs.
- 13 unit tests cover all branches: step routing for N in {1,2,3}, backspace navigation, criteria multi-entry, submit assembly, validation errors, non-press guard, error cleared on next press.

## Verification

```
cargo test -p assay-tui
# 13 unit tests + 1 integration test — all passed

cargo fmt --check -p assay-tui
# clean (ran cargo fmt to fix style)

cargo clippy -p assay-tui -- -D warnings
# 0 errors/warnings in assay-tui (pre-existing derivable_impls warning in assay-types is unrelated)
```

The `wizard_round_trip` integration test drives N=2 chunks through all steps with synthetic KeyEvents, calls `create_from_inputs`, and asserts milestone TOML + two `gates.toml` files exist in a tempdir.

## Diagnostics

- `state.error: Option<String>` — set on validation failure (empty name, invalid chunk count); cleared on next press
- `cargo test -p assay-tui -- --nocapture` — shows per-test step traces
- Integration test confirms real filesystem writes via `create_from_inputs`

## Deviations

None — implemented exactly as specified in T02-PLAN.md.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-tui/src/wizard.rs` — full state machine implementation replacing all `todo!("T02")` stubs; 13 unit tests added
