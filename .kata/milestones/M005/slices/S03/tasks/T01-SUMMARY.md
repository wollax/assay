---
id: T01
parent: S03
milestone: M005
provides:
  - "Contract tests for assay_core::wizard (5 tests)"
  - "Contract tests for MCP milestone_create and spec_create tools (5 tests)"
key_files:
  - crates/assay-core/tests/wizard.rs
  - crates/assay-mcp/src/server.rs
key_decisions:
  - "create_spec_from_params takes (slug, name, milestone_slug: Option<&str>, assay_dir, specs_dir) — matches MCP SpecCreateParams shape"
  - "WizardInputs uses Vec<WizardChunkInput>; each WizardChunkInput has slug, name, criteria: Vec<String>"
  - "MilestoneChunkInput mirrors WizardChunkInput for the MCP layer"
patterns_established:
  - "Wizard core tests use TempDir::new() (not tempdir()) for isolated paths without set_current_dir"
  - "MCP wizard tests follow same create_project + set_current_dir pattern as existing cycle/milestone tests"
observability_surfaces:
  - none
duration: short
verification_result: passed
completed_at: 2026-03-20
blocker_discovered: false
---

# T01: Write Failing Wizard Core + MCP Integration Tests

**Wrote 5 failing wizard-core contract tests and 5 failing MCP wizard tool contract tests; both fail to compile exactly as intended.**

## What Happened

Created `crates/assay-core/tests/wizard.rs` with 5 integration tests covering the full `assay_core::wizard` API contract:
- `wizard_create_from_inputs_writes_files` — verifies milestone TOML + two `gates.toml` files are written
- `wizard_create_from_inputs_sets_milestone_and_order_on_specs` — verifies `milestone` and `order` fields are set correctly (0 and 1)
- `wizard_slug_collision_returns_error` — verifies second call with same slug returns `Err`
- `wizard_create_spec_patches_milestone` — sets up milestone via `milestone_save`, calls `create_spec_from_params`, reloads milestone and checks `chunks` contains the new slug
- `wizard_create_spec_rejects_nonexistent_milestone` — verifies `Err` for `milestone_slug = Some("ghost")` with no backing file

Added 5 new test functions to the `#[cfg(test)]` block in `crates/assay-mcp/src/server.rs`:
- `milestone_create_tool_in_router` — router registration check
- `spec_create_tool_in_router` — router registration check
- `milestone_create_writes_milestone_toml` — end-to-end: calls `server.milestone_create(...)`, asserts `!is_error` and `.assay/milestones/test-ms.toml` exists
- `spec_create_writes_gates_toml` — end-to-end: calls `server.spec_create(...)`, asserts `.assay/specs/chunk-1/gates.toml` exists
- `spec_create_rejects_duplicate` — second call with same slug must return `is_error: true`

## Verification

Compile errors verified as expected:

```
# wizard core: exactly one error
cargo test -p assay-core --features assay-types/orchestrate --test wizard 2>&1 | grep "error\[E"
→ error[E0432]: unresolved import `assay_core::wizard`

# MCP wizard tools: missing params structs + methods
cargo test -p assay-mcp -- milestone_create 2>&1 | grep "error\[E"
→ error[E0422]: cannot find struct, variant or union type `MilestoneCreateParams` in this scope
→ error[E0422]: cannot find struct, variant or union type `MilestoneChunkInput` in this scope
→ error[E0422]: cannot find struct, variant or union type `SpecCreateParams` in this scope
→ error[E0599]: no method named `milestone_create` found for struct `server::AssayServer`
→ error[E0599]: no method named `spec_create` found for struct `server::AssayServer`
```

No warnings in the wizard.rs compile attempt after removing the unused `ChunkRef` import.

## Diagnostics

Run `cargo test -p assay-core --features assay-types/orchestrate --test wizard` to see exact missing types/functions. Each error message names the missing symbol T02 must implement.

## Deviations

- Removed `ChunkRef` from `use assay_types::{...}` in wizard.rs — it was imported speculatively but not used in any test body (we manipulate the milestone via `milestone_save`/`milestone_load`, not by constructing `ChunkRef` directly). No impact on contract coverage.

## Known Issues

None. Tests will compile cleanly once T02 adds `assay_core::wizard` and T04 adds the MCP params/methods.

## Files Created/Modified

- `crates/assay-core/tests/wizard.rs` — 5 new contract tests for the wizard core API
- `crates/assay-mcp/src/server.rs` — 5 new contract tests for `milestone_create` and `spec_create` MCP tools
