---
id: T01
parent: S04
milestone: M013
provides:
  - CriterionInput derives Debug
  - WizardChunkInput.criteria is Vec<CriterionInput> (was Vec<String>)
  - write_gates_toml threads cmd from CriterionInput to Criterion
  - create_spec_from_params accepts Vec<CriterionInput>
  - Contract tests proving cmd round-trip and cmd-None omission
key_files:
  - crates/assay-core/src/wizard.rs
  - crates/assay-core/tests/wizard.rs
key_decisions:
  - "CriterionInput description field threaded through (not just name+cmd) for completeness"
patterns_established:
  - "CriterionInput is the canonical wizard input type for criteria across all surfaces"
observability_surfaces:
  - Generated gates.toml cmd field presence/absence is the observable contract
duration: 8min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T01: Wire CriterionInput through core wizard and write contract tests

**Changed `WizardChunkInput.criteria` from `Vec<String>` to `Vec<CriterionInput>` and added contract tests proving cmd round-trips through generated `gates.toml`.**

## What Happened

Added `#[derive(Debug)]` to `CriterionInput`. Changed `WizardChunkInput.criteria` from `Vec<String>` to `Vec<CriterionInput>`. Updated `write_gates_toml` to construct `Criterion` from `CriterionInput` fields (name, description, cmd) instead of hardcoding empty description and None cmd. Updated `create_spec_from_params` signature to accept `Vec<CriterionInput>`. Updated test helper `one_criterion_chunk` to build `CriterionInput` structs. Added two new contract tests: one proving `cmd: Some("cargo test")` round-trips to the generated TOML, another proving `cmd: None` produces no `cmd` key.

## Verification

- `cargo test -p assay-core --test wizard` — 7/7 tests pass (5 existing + 2 new)
- `cargo check -p assay-core` — core crate compiles clean
- `cargo check --workspace` — expected failures in `assay-tui` and `assay-mcp` (they still pass `Vec<String>`, fixed in T02/T03)

### Slice-level checks:
- ✅ `cargo test -p assay-core --test wizard` — all pass
- ⏳ `cargo test -p assay-tui --test wizard_round_trip` — blocked until T02
- ⏳ `cargo test -p assay-mcp` — blocked until T03
- ⏳ `just ready` — blocked until T02/T03 fix downstream callers

## Diagnostics

Inspect generated `gates.toml` files — `cmd` field presence/absence is the observable contract. Compile errors in downstream crates (`assay-tui`, `assay-mcp`) are the fail-fast signal that callers haven't been updated yet.

## Deviations

None.

## Known Issues

Downstream crates (`assay-tui`, `assay-mcp`) don't compile until T02/T03 update their callers — this is by design (fail-fast type cascading).

## Files Created/Modified

- `crates/assay-core/src/wizard.rs` — Added Debug derive to CriterionInput; changed WizardChunkInput.criteria type; updated write_gates_toml and create_spec_from_params signatures
- `crates/assay-core/tests/wizard.rs` — Updated helper to use CriterionInput; added cmd round-trip and cmd-None tests
