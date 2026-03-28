# S04: Wizard runnable criteria

**Goal:** Thread an optional `cmd` field through the wizard flow so wizard-generated specs can run `gate run` immediately without manual editing.
**Demo:** `assay plan` wizard prompts for an optional command per criterion; generated `gates.toml` has `cmd` field when provided; `gate run` succeeds on wizard output without manual editing.

## Must-Haves

- `WizardChunkInput.criteria` changed from `Vec<String>` to `Vec<CriterionInput>` (existing struct with `name`, `description`, `cmd`)
- `write_gates_toml` passes `cmd` through to `Criterion` construction
- `create_spec_from_params` accepts `Vec<CriterionInput>` instead of `Vec<String>`
- CLI wizard prompts for optional cmd after each criterion name (Enter skips → `None`)
- MCP `spec_create` and `milestone_create` criteria params accept objects with optional `cmd` (backward-compatible with plain strings via untagged enum)
- TUI wizard collects optional cmd per criterion in the criteria step
- Contract test: wizard round-trip produces gates.toml with `cmd` field set
- All existing tests updated and passing

## Proof Level

- This slice proves: contract + integration
- Real runtime required: no (contract tests with filesystem round-trip)
- Human/UAT required: yes — interactive TTY wizard and TUI form require manual testing

## Verification

- `cargo test -p assay-core --test wizard` — core wizard round-trip tests including cmd field
- `cargo test -p assay-tui --test wizard_round_trip` — TUI wizard state machine with cmd sub-step
- `cargo test -p assay-mcp` — MCP spec_create and milestone_create with new criteria format
- `just ready` — all 1516+ tests pass, no regressions

## Observability / Diagnostics

- Runtime signals: None (wizard is a pure authoring flow; no runtime events)
- Inspection surfaces: Generated `gates.toml` files (inspect `cmd` field presence/absence)
- Failure visibility: Compile errors on type mismatch (Vec<String> → Vec<CriterionInput> cascades through all callers)
- Redaction constraints: None

## Integration Closure

- Upstream surfaces consumed: `assay_core::wizard::CriterionInput` (existing unused struct), `assay_types::Criterion.cmd` (already `Option<String>` with serde skip-if-None)
- New wiring introduced in this slice: `CriterionInput` wired from all three UI surfaces (CLI, TUI, MCP) through `write_gates_toml` to `Criterion.cmd` in generated TOML
- What remains before the milestone is truly usable end-to-end: nothing — this is the last slice in M013

## Tasks

- [x] **T01: Wire CriterionInput through core wizard and write contract tests** `est:30m`
  - Why: The type change from `Vec<String>` to `Vec<CriterionInput>` is the foundation; all UI surfaces depend on it. Contract tests prove the cmd field round-trips to gates.toml.
  - Files: `crates/assay-core/src/wizard.rs`, `crates/assay-core/tests/wizard.rs`
  - Do: Add `Debug` to `CriterionInput`. Change `WizardChunkInput.criteria` to `Vec<CriterionInput>`. Update `write_gates_toml` to accept `&[CriterionInput]` and pass `cmd` through. Update `create_spec_from_params` to accept `Vec<CriterionInput>`. Update `create_from_inputs` callers. Fix all existing core wizard tests to use `CriterionInput`. Add new test asserting `cmd` field appears in generated TOML.
  - Verify: `cargo test -p assay-core --test wizard` passes; new cmd round-trip test passes
  - Done when: `write_gates_toml` writes `cmd` when `Some`; omits it when `None`; existing tests updated and green

- [ ] **T02: Update CLI wizard and MCP tools for cmd input** `est:30m`
  - Why: CLI and MCP are the two non-TUI input surfaces. CLI needs an interactive cmd prompt; MCP needs backward-compatible schema change.
  - Files: `crates/assay-cli/src/commands/plan.rs`, `crates/assay-mcp/src/server.rs`
  - Do: CLI: after each criterion name prompt, add `dialoguer::Input::allow_empty(true)` for cmd ("  Command (Enter to skip):"). Build `CriterionInput` from name + cmd. MCP: create `CriterionInputParam` struct with `name`, optional `description`, optional `cmd`. Create untagged enum `CriterionOrString` that accepts either a plain string or a `CriterionInputParam` object. Update `SpecCreateParams.criteria` and `MilestoneChunkInput.criteria` to use `Vec<CriterionOrString>`. Add conversion from `CriterionOrString` to `CriterionInput`. Update MCP handler call sites. Update MCP tests.
  - Verify: `cargo test -p assay-cli` passes; `cargo test -p assay-mcp` passes; MCP tests cover both string and object criteria formats
  - Done when: CLI wizard collects cmd per criterion; MCP accepts both plain strings and objects with cmd

- [ ] **T03: Update TUI wizard for cmd collection and verify all surfaces** `est:30m`
  - Why: TUI is the third input surface. The criteria step must collect cmd per criterion. This task also runs the full verification suite.
  - Files: `crates/assay-tui/src/wizard.rs`, `crates/assay-tui/tests/wizard_round_trip.rs`
  - Do: Modify TUI wizard criteria step to use a two-phase input: after each criterion name (non-empty Enter), prompt for cmd (next Enter). Track whether current line is a name or cmd via a `bool` flag on `WizardState`. Empty name line still ends criteria collection. Build `CriterionInput` from name+cmd pairs in `assemble_inputs`. Update `draw_wizard` to show "Command (Enter to skip):" prompt during cmd sub-step. Update wizard_round_trip tests to drive through cmd steps. Add test asserting cmd appears in assembled `WizardInputs`.
  - Verify: `cargo test -p assay-tui --test wizard_round_trip` passes; `just ready` green with all 1516+ tests
  - Done when: TUI wizard collects cmd per criterion; full workspace passes

## Files Likely Touched

- `crates/assay-core/src/wizard.rs`
- `crates/assay-core/tests/wizard.rs`
- `crates/assay-cli/src/commands/plan.rs`
- `crates/assay-mcp/src/server.rs`
- `crates/assay-tui/src/wizard.rs`
- `crates/assay-tui/tests/wizard_round_trip.rs`
