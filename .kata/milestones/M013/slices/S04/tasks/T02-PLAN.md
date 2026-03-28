---
estimated_steps: 5
estimated_files: 2
---

# T02: Update CLI wizard and MCP tools for cmd input

**Slice:** S04 — Wizard runnable criteria
**Milestone:** M013

## Description

Update the CLI `assay plan` wizard to prompt for an optional command after each criterion name, and update the MCP `spec_create` and `milestone_create` tools to accept criteria as objects with an optional `cmd` field while remaining backward-compatible with plain strings.

## Steps

1. In `crates/assay-cli/src/commands/plan.rs`: after collecting each criterion name, add an optional cmd prompt using `dialoguer::Input::new().with_prompt("    Command (Enter to skip):").allow_empty(true).interact_text()`. Convert empty string to `None`. Build `CriterionInput { name: criterion_name, description: String::new(), cmd }` and push to criteria vec. Update the `chunks` vec construction to use the new criteria type.
2. In `crates/assay-mcp/src/server.rs`: create a new `CriterionInputParam` struct with `name: String`, `description: Option<String>`, `cmd: Option<String>` (derives `Deserialize, JsonSchema`). Create an untagged enum `CriterionOrString` with variants `Object(CriterionInputParam)` and `Plain(String)` (derives `Deserialize, JsonSchema`, uses `#[serde(untagged)]`). Add an `impl From<CriterionOrString> for assay_core::wizard::CriterionInput` conversion.
3. Update `SpecCreateParams.criteria` from `Vec<String>` to `Vec<CriterionOrString>`. Update `MilestoneChunkInput.criteria` from `Vec<String>` to `Vec<CriterionOrString>`.
4. Update the `spec_create` handler: convert `params.0.criteria` from `Vec<CriterionOrString>` to `Vec<CriterionInput>` via `.into_iter().map(Into::into).collect()` before passing to `create_spec_from_params`.
5. Update the `milestone_create` handler: convert each `MilestoneChunkInput` to `WizardChunkInput` with criteria mapped through the `CriterionOrString → CriterionInput` conversion. Update existing MCP tests to pass `CriterionOrString::Plain(...)` or the object form. Add a test that passes an object with `cmd` and verifies the generated `gates.toml` has the `cmd` field.

## Must-Haves

- [ ] CLI wizard prompts for cmd after each criterion name; Enter-to-skip produces `cmd: None`
- [ ] `CriterionOrString` untagged enum accepts both `"string"` and `{"name": "...", "cmd": "..."}` JSON
- [ ] `SpecCreateParams.criteria` and `MilestoneChunkInput.criteria` use `Vec<CriterionOrString>`
- [ ] Existing MCP tests pass with plain string criteria (backward compat)
- [ ] New MCP test passes object criteria with cmd field

## Verification

- `cargo test -p assay-cli` — plan_non_tty_returns_1 still passes
- `cargo test -p assay-mcp` — all MCP tests pass including new cmd-aware test
- `cargo check --workspace` — compiles (TUI may still fail until T03)

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: MCP JSON schema documents the criteria format (string or object)
- Failure state exposed: serde deserialization error on malformed criteria input

## Inputs

- `crates/assay-core/src/wizard.rs` — T01's `CriterionInput` type and updated `create_spec_from_params` signature
- D005: MCP tools are additive only — changing criteria type is a schema change but backward-compatible via untagged enum
- D178: cmd is optional per-criterion

## Expected Output

- `crates/assay-cli/src/commands/plan.rs` — cmd prompt after each criterion; builds `CriterionInput`
- `crates/assay-mcp/src/server.rs` — `CriterionInputParam`, `CriterionOrString`, updated params, updated handlers, new test
