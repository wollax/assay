---
id: S04
parent: M013
milestone: M013
provides:
  - CriterionInput derives Debug and is the canonical wizard input type across all surfaces
  - WizardChunkInput.criteria changed from Vec<String> to Vec<CriterionInput>
  - write_gates_toml threads cmd from CriterionInput to Criterion; omits field when None
  - create_spec_from_params accepts Vec<CriterionInput>
  - CLI assay plan wizard prompts for optional cmd after each criterion (Enter skips → None)
  - CriterionOrString untagged enum enables backward-compatible MCP criteria input (string or object)
  - SpecCreateParams.criteria and MilestoneChunkInput.criteria accept Vec<CriterionOrString>
  - TUI wizard criteria step alternates name→cmd sub-steps via criteria_awaiting_cmd flag
  - assemble_inputs pairs name/cmd entries into CriterionInput structs
  - Contract tests proving cmd round-trips and cmd-None omission in all surfaces
requires: []
affects: []
key_files:
  - crates/assay-core/src/wizard.rs
  - crates/assay-core/tests/wizard.rs
  - crates/assay-cli/src/commands/plan.rs
  - crates/assay-mcp/src/server.rs
  - crates/assay-tui/src/wizard.rs
  - crates/assay-tui/tests/wizard_round_trip.rs
  - crates/assay-tui/tests/app_wizard.rs
key_decisions:
  - "CriterionInput description field threaded through (not just name+cmd) — complete criterion data from wizard to TOML"
  - "CriterionOrString uses serde untagged enum for MCP backward compatibility — existing agent callers using plain strings continue to work"
  - "criteria_awaiting_cmd bool flag within single wizard step index — no step arithmetic changes (per S04-RESEARCH pitfall)"
patterns_established:
  - "CriterionInput is the canonical wizard input type for criteria across all surfaces (CLI, TUI, MCP)"
  - "CriterionOrString is the MCP-facing input type; CriterionInput remains the core domain type"
  - "Sub-step flags (criteria_awaiting_cmd) for multi-phase input within a single wizard step index"
observability_surfaces:
  - Generated gates.toml cmd field presence/absence is the observable contract
  - MCP JSON schema documents the criteria format (string or object) via schemars
  - WizardState.criteria_awaiting_cmd is a public bool — inspectable in tests
drill_down_paths:
  - .kata/milestones/M013/slices/S04/tasks/T01-SUMMARY.md
  - .kata/milestones/M013/slices/S04/tasks/T02-SUMMARY.md
  - .kata/milestones/M013/slices/S04/tasks/T03-SUMMARY.md
duration: 30min
verification_result: passed
completed_at: 2026-03-28T13:00:00Z
---

# S04: Wizard runnable criteria

**Threaded optional `cmd` field from wizard input through all three surfaces (CLI, TUI, MCP) to `gates.toml` generation, so wizard-created specs run `gate run` immediately without manual editing.**

## What Happened

**T01** laid the foundation: added `#[derive(Debug)]` to `CriterionInput`, changed `WizardChunkInput.criteria` from `Vec<String>` to `Vec<CriterionInput>`, and updated `write_gates_toml` and `create_spec_from_params` accordingly. Two new contract tests confirm the round-trip: `cmd: Some("cargo test")` produces a `cmd` key in the generated TOML; `cmd: None` produces no `cmd` key. This intentionally broke downstream crates (assay-tui, assay-mcp) as a fail-fast cascade.

**T02** updated CLI and MCP surfaces. The CLI `assay plan` wizard gained an optional command prompt after each criterion name using `dialoguer::Input::allow_empty(true)` — empty Enter maps to `cmd: None`. The MCP surface introduced `CriterionInputParam` (structured input: name + optional description + optional cmd), `CriterionOrString` (`#[serde(untagged)]` enum accepting either format), and updated `SpecCreateParams.criteria` and `MilestoneChunkInput.criteria` to `Vec<CriterionOrString>`. Five new MCP tests cover all permutations; four existing tests were updated.

**T03** completed the TUI surface. A `criteria_awaiting_cmd: bool` flag was added to `WizardState` to track name→cmd alternation within the existing criteria step (no step index arithmetic changes). After a non-empty criterion name, the wizard switches to cmd sub-step; after cmd (or blank Enter to skip), it resets and awaits the next name. `assemble_inputs` was updated to iterate criteria fields in pairs. `step_prompt` now returns "Command (Enter to skip):" during cmd sub-step. The `wizard_round_trip.rs` and `app_wizard.rs` test drivers were updated for the new flow, and two new tests assert cmd-specific behavior.

## Verification

- `cargo test -p assay-core --test wizard` — 7/7 pass (5 existing + 2 new cmd round-trip tests)
- `cargo test -p assay-tui --test wizard_round_trip` — 11/11 pass (9 existing + 2 new cmd tests)
- `cargo test -p assay-mcp` — 162/162 pass (all new and existing)
- `cargo test -p assay-cli` — 52/52 pass
- `just ready` — 1525 tests pass; fmt/clippy/deny all green

## Requirements Advanced

- R082 — wizard now collects optional cmd per criterion across all three surfaces; `gates.toml` writes the `cmd` field when provided

## Requirements Validated

- R082 — contract tests prove the full round-trip: wizard input → `cmd: Option<String>` → gates.toml `cmd` field presence/absence; `gate run` succeeds immediately on wizard output without manual editing; `just ready` green with 1525 tests

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- T03 also updated `crates/assay-tui/tests/app_wizard.rs` — not listed in the original task plan but required because it used the old criteria-step protocol. This was the expected cascading failure from changing the criteria step contract.

## Known Limitations

- Interactive CLI wizard (TTY path) and TUI wizard form require human testing — UAT only
- `cmd` is purely optional; users who leave it blank get the same text-only output as before

## Follow-ups

- none — this is the last slice in M013; milestone is complete

## Files Created/Modified

- `crates/assay-core/src/wizard.rs` — Added Debug derive to CriterionInput; changed WizardChunkInput.criteria type; updated write_gates_toml and create_spec_from_params signatures
- `crates/assay-core/tests/wizard.rs` — Updated helper to use CriterionInput; added cmd round-trip and cmd-None tests
- `crates/assay-cli/src/commands/plan.rs` — Added optional cmd prompt after criterion name; builds CriterionInput directly
- `crates/assay-mcp/src/server.rs` — Added CriterionInputParam, CriterionOrString types; updated params; updated spec_create handler; 5 new tests, 4 updated
- `crates/assay-tui/src/wizard.rs` — Added criteria_awaiting_cmd flag; updated enter handler for name→cmd alternation; updated assemble_inputs; updated step_prompt
- `crates/assay-tui/tests/wizard_round_trip.rs` — Updated drive_two_chunk_wizard for cmd sub-steps; updated assertions; added cmd-specific tests
- `crates/assay-tui/tests/app_wizard.rs` — Updated drive_two_chunk_wizard helper for cmd sub-step flow

## Forward Intelligence

### What the next slice should know
- M013 is now complete — all four slices (S01–S04) done; the milestone is ready to close
- `CriterionInput` is now the canonical type for wizard criteria input across all surfaces; future criteria extensions should go here first

### What's fragile
- TUI `criteria_awaiting_cmd` flag interacts with the backspace-to-prev-step logic — any future changes to WizardState step handling should confirm the cmd sub-step flag resets correctly on backward navigation

### Authoritative diagnostics
- Generated `gates.toml` files under `.assay/specs/*/gates.toml` — `cmd` field presence is the observable contract
- `WizardState.criteria_awaiting_cmd` public bool — assert directly in TUI wizard tests

### What assumptions changed
- D076 assumed cmd collection was deferred; R082 explicitly revisited and superseded it — cmd collection is now implemented
