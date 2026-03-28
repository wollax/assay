---
estimated_steps: 5
estimated_files: 2
---

# T01: Wire CriterionInput through core wizard and write contract tests

**Slice:** S04 — Wizard runnable criteria
**Milestone:** M013

## Description

Change `WizardChunkInput.criteria` from `Vec<String>` to `Vec<CriterionInput>` and update all core wizard functions to thread the `cmd` field through to the generated `gates.toml`. This is the foundational type change that cascades to all UI surfaces. Write contract tests proving cmd round-trips correctly.

## Steps

1. Add `#[derive(Debug)]` to `CriterionInput` in `wizard.rs` (it's missing; `WizardChunkInput` derives `Debug` but contains `criteria`).
2. Change `WizardChunkInput.criteria` from `Vec<String>` to `Vec<CriterionInput>`. Update the doc comment.
3. Update `write_gates_toml` signature: change `criteria_names: &[String]` to `criteria: &[CriterionInput]`. In the body, construct `Criterion` using `input.name`, `input.description`, and `input.cmd` instead of hardcoded values.
4. Update `create_spec_from_params` to accept `criteria: Vec<CriterionInput>` instead of `Vec<String>`. Thread through to `write_gates_toml`.
5. Update all existing tests in `crates/assay-core/tests/wizard.rs`: change `one_criterion_chunk` helper to build `CriterionInput` structs. Add a new test `wizard_cmd_field_round_trips_to_gates_toml` that creates a chunk with `cmd: Some("cargo test".to_string())`, calls `create_from_inputs`, reads the generated `gates.toml`, and asserts the `cmd` field is present. Add a complementary test asserting `cmd: None` produces no `cmd` field in the TOML.

## Must-Haves

- [ ] `CriterionInput` derives `Debug`
- [ ] `WizardChunkInput.criteria` is `Vec<CriterionInput>`
- [ ] `write_gates_toml` passes `cmd` from `CriterionInput` to `Criterion`
- [ ] `create_spec_from_params` accepts `Vec<CriterionInput>`
- [ ] New test proves `cmd: Some("cargo test")` round-trips to `gates.toml` with `cmd = "cargo test"`
- [ ] New test proves `cmd: None` produces no `cmd` key in generated TOML
- [ ] All existing `cargo test -p assay-core --test wizard` tests pass

## Verification

- `cargo test -p assay-core --test wizard` — all tests pass including new cmd round-trip tests
- `cargo check --workspace` — no compile errors from downstream crates (will fail until T02/T03 fix them, but core crate itself compiles)

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: Read generated `gates.toml` file; `cmd` field presence/absence is the observable contract
- Failure state exposed: Compile error if any caller still passes `Vec<String>` — fail-fast by design

## Inputs

- `crates/assay-core/src/wizard.rs` — existing `CriterionInput` struct (unused), `WizardChunkInput`, `write_gates_toml`, `create_spec_from_params`
- D178: cmd is optional and per-criterion; empty input skips cmd

## Expected Output

- `crates/assay-core/src/wizard.rs` — `CriterionInput` derives Debug; `WizardChunkInput.criteria` is `Vec<CriterionInput>`; `write_gates_toml` and `create_spec_from_params` accept the new type
- `crates/assay-core/tests/wizard.rs` — updated helpers; 2+ new tests for cmd round-trip
