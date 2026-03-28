---
id: T02
parent: S04
milestone: M013
provides:
  - CLI wizard prompts for optional cmd per criterion (Enter-to-skip → None)
  - CriterionOrString untagged enum for MCP backward-compatible criteria input
  - CriterionInputParam struct for structured MCP criterion objects
  - SpecCreateParams.criteria and MilestoneChunkInput.criteria accept Vec<CriterionOrString>
  - From<CriterionOrString> for CriterionInput conversion
key_files:
  - crates/assay-cli/src/commands/plan.rs
  - crates/assay-mcp/src/server.rs
key_decisions:
  - "CriterionOrString uses serde untagged enum for backward compatibility — plain strings and objects both accepted"
patterns_established:
  - "CriterionOrString is the MCP-facing input type for criteria; CriterionInput remains the core domain type"
observability_surfaces:
  - MCP JSON schema documents the criteria format (string or object) via schemars
  - serde deserialization error on malformed criteria input
duration: 12min
verification_result: passed
completed_at: 2026-03-28T12:00:00Z
blocker_discovered: false
---

# T02: Update CLI wizard and MCP tools for cmd input

**CLI wizard now prompts for optional command per criterion; MCP tools accept both plain strings and structured objects with cmd field via untagged enum**

## What Happened

Updated the CLI `assay plan` wizard to prompt for an optional shell command after each criterion name using `dialoguer::Input` with `allow_empty(true)`. Empty input maps to `cmd: None`, producing `CriterionInput` directly instead of plain strings.

For the MCP surface, introduced three types:
- `CriterionInputParam` — structured input with `name`, optional `description`, optional `cmd`
- `CriterionOrString` — `#[serde(untagged)]` enum with `Object(CriterionInputParam)` and `Plain(String)` variants
- `From<CriterionOrString> for CriterionInput` conversion

Updated `SpecCreateParams.criteria` and `MilestoneChunkInput.criteria` from `Vec<String>` to `Vec<CriterionOrString>`. Updated the `spec_create` handler to convert through `.into_iter().map(Into::into).collect()`. The `milestone_create` handler doesn't use criteria directly (only slug/order pairs), so no handler change needed there.

Added 5 new tests: deserialization of plain strings, objects with cmd, objects without cmd, mixed vectors, and an integration test verifying the generated `gates.toml` contains the `cmd` field. Updated 4 existing tests to use `CriterionOrString::Plain(...)`.

## Verification

- `cargo test -p assay-cli` — 52 passed including `plan_non_tty_returns_1`
- `cargo test -p assay-mcp` — 162 passed (131 unit + 31 integration) including all new and existing tests
- `cargo check --workspace` — compiles (TUI fails as expected, awaiting T03)

## Diagnostics

- MCP JSON schema via schemars documents the criteria format for agent callers
- Malformed criteria JSON produces a serde deserialization error

## Deviations

None.

## Known Issues

- `assay-tui` crate doesn't compile due to `Vec<String>` vs `Vec<CriterionInput>` mismatch — expected, will be fixed in T03.

## Files Created/Modified

- `crates/assay-cli/src/commands/plan.rs` — Added cmd prompt after criterion name; builds CriterionInput directly
- `crates/assay-mcp/src/server.rs` — Added CriterionInputParam, CriterionOrString types; updated params; updated spec_create handler; added 5 new tests; updated 4 existing tests
