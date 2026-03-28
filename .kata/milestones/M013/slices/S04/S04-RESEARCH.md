# S04: Wizard runnable criteria — Research

**Date:** 2026-03-28

## Summary

S04 adds an optional `cmd` field per criterion to the wizard flow, making wizard-generated specs immediately runnable via `gate run` without manual editing. The work spans four modules: `assay-core::wizard` (core logic), `assay-cli::commands::plan` (CLI dialoguer wizard), `assay-tui::wizard` (TUI state machine), and `assay-mcp::server` (MCP `spec_create`/`milestone_create` tools).

The `Criterion` type in `assay-types` already has `cmd: Option<String>` with proper serde skip-if-None semantics. The `write_gates_toml` helper in `wizard.rs` already constructs `Criterion` structs and explicitly sets `cmd: None`. The entire change is about threading `cmd` data from user input through to that construction site.

The primary recommendation is to change `WizardChunkInput.criteria` from `Vec<String>` to `Vec<CriterionInput>` (the `CriterionInput` struct already exists in `wizard.rs` with `name`, `description`, `cmd` fields but is currently unused). This propagates through all callers: CLI dialoguer loop, TUI wizard state machine, MCP params, and tests.

## Recommendation

1. **Use the existing `CriterionInput` struct** — `assay_core::wizard::CriterionInput` already has `name: String`, `description: String`, `cmd: Option<String>`. It was added but never wired up.
2. **Change `WizardChunkInput.criteria` from `Vec<String>` to `Vec<CriterionInput>`** — this is the single type change that cascades through all surfaces.
3. **Update `write_gates_toml`** to accept `&[CriterionInput]` instead of `&[String]` and pass through the `cmd` field.
4. **Update `create_spec_from_params`** to accept `Vec<CriterionInput>` instead of `Vec<String>`.
5. **CLI wizard**: after each criterion name prompt, add an optional cmd prompt (Enter skips → `None`).
6. **TUI wizard**: add a cmd sub-step after each criterion entry — intercalate cmd prompts within the criteria step.
7. **MCP params**: change `SpecCreateParams.criteria` and `MilestoneChunkInput.criteria` from `Vec<String>` to a new MCP-visible struct with optional `cmd`.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Criterion type with optional cmd | `assay_types::Criterion` already has `cmd: Option<String>` | Schema is locked; serde skip-if-None already works |
| Wizard criterion input struct | `assay_core::wizard::CriterionInput` exists with `name`, `description`, `cmd` | Already defined; just needs to be wired up |
| Atomic TOML writes | `write_gates_toml()` in `wizard.rs` uses `NamedTempFile + persist` | Reuse; only change the `Criterion` construction to pass `cmd` through |

## Existing Code and Patterns

- `crates/assay-core/src/wizard.rs` — **Primary target.** Contains `CriterionInput` (unused), `WizardChunkInput` (criteria as `Vec<String>`), `create_from_inputs`, `create_spec_from_params`, and `write_gates_toml`. The `write_gates_toml` helper constructs `Criterion` with `cmd: None` — change this to use the input's cmd.
- `crates/assay-cli/src/commands/plan.rs` — CLI wizard using `dialoguer`. Currently collects criterion names in a loop. Add an optional cmd prompt after each criterion name (Enter-to-skip pattern: `dialoguer::Input::allow_empty(true)`).
- `crates/assay-tui/src/wizard.rs` — TUI wizard state machine with step-based navigation. Currently alternates chunk-name / chunk-criteria steps. The criteria step collects multi-line entries (one per criterion). Need to add a cmd sub-step per criterion.
- `crates/assay-mcp/src/server.rs` — `SpecCreateParams.criteria: Vec<String>` and `MilestoneChunkInput.criteria: Vec<String>`. Need new MCP-visible struct for criteria with optional cmd.
- `crates/assay-types/src/criterion.rs` — `Criterion` struct with `cmd: Option<String>` already serializes/deserializes correctly. No changes needed here.
- `crates/assay-core/tests/wizard.rs` — Integration tests. Uses `Vec<String>` for criteria in helpers. Must update to use `CriterionInput`.
- `crates/assay-tui/tests/wizard_round_trip.rs` — TUI wizard integration test. Will need updating for the new cmd sub-step.

## Constraints

- **D178 (decision already recorded):** cmd is optional and per-criterion; empty input skips cmd. Supersedes D076.
- **D005:** MCP tools are additive only — changing `spec_create`'s `criteria` type from `Vec<String>` to `Vec<CriterionInputParam>` is technically a schema change. However, since the tool is still relatively new and the change is backward-compatible (agents can pass objects with just `name`), this is acceptable. Add `description` as `Option<String>` too.
- **D074:** Tests are the authoritative contract — write tests first, implementation follows.
- **D094:** TUI wizard chunk-count uses replace semantics, not append. Any new steps must follow existing event-handler patterns.
- **D076 → D178:** The decision to make cmd optional is already recorded.
- **`deny_unknown_fields` on `Criterion`** — no new fields needed on `Criterion` itself; `cmd` already exists.
- **`WizardChunkInput` is `Debug` but not `Serialize`/`Deserialize`** — so changing its `criteria` field type has no schema implications.

## Common Pitfalls

- **TUI wizard state machine complexity** — The current state machine uses step indices with `offset / 2` and `offset % 2` arithmetic. Adding a cmd sub-step per criterion changes the step count dynamically (it depends on how many criteria were entered). Two approaches: (a) collect cmd inline within the criteria step using a two-phase input per criterion (name → cmd → name → cmd → blank), or (b) add a separate cmd collection pass after all criteria for a chunk. Approach (a) is more natural UX but requires careful state management. Approach (b) is simpler but less intuitive. **Recommendation: approach (a)** — use the existing multi-line criteria step but alternate between criterion-name and criterion-cmd lines. Track whether the current line is a name or a cmd via a bool or sub-step counter.
- **MCP backward compatibility** — Current `criteria: Vec<String>` is the simplest form. Changing to `Vec<CriterionInputParam>` (an object) breaks agents that pass plain strings. **Mitigation:** Use `#[serde(untagged)]` on an enum that accepts either a plain string or an object, OR keep `criteria: Vec<String>` and add a new `criteria_v2: Option<Vec<CriterionInputParam>>` field. **Recommendation:** Use untagged enum to accept both formats — simpler for callers, no field duplication.
- **CLI wizard UX** — Adding a cmd prompt after every criterion could feel heavy if the user doesn't want commands. `dialoguer::Input::allow_empty(true)` with a prompt like "  Command (Enter to skip):" keeps it lightweight.
- **Test update scope** — Both `crates/assay-core/tests/wizard.rs` and `crates/assay-tui/tests/wizard_round_trip.rs` construct `WizardChunkInput` directly. Changing the `criteria` field type breaks them at compile time — good (fail-fast), but all tests must be updated simultaneously.

## Open Risks

- **TUI wizard step index arithmetic** — The current `(step - 3) / 2` and `(step - 3) % 2` pattern for alternating chunk-name/criteria is fragile. Adding cmd collection within the criteria step requires careful sub-state tracking. Risk: off-by-one errors in step navigation, especially with backspace. Mitigation: thorough integration tests covering forward/backward navigation through the new cmd flow.
- **Untagged enum serde ambiguity** — If using `#[serde(untagged)]` for MCP criteria (string vs object), the JSON `"cargo test"` (string) becomes a name-only criterion. An object `{"name": "tests pass", "cmd": "cargo test"}` provides the full form. The ambiguity is well-defined but must be documented for MCP consumers.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| dialoguer | — | Already in use (v0.12.0); no skill needed |
| ratatui | — | Already in use; TUI patterns established in codebase |

No external skills are relevant for this slice — it's entirely internal codebase work on existing patterns.

## Sources

- Codebase analysis of `assay-core::wizard`, `assay-cli::commands::plan`, `assay-tui::wizard`, `assay-mcp::server`
- Decision register: D076, D178, D074, D005, D094
