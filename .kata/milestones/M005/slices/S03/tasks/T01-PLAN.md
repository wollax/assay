---
estimated_steps: 5
estimated_files: 2
---

# T01: Write Failing Wizard Core + MCP Integration Tests

**Slice:** S03 — Guided Authoring Wizard
**Milestone:** M005

## Description

Write integration test files that define the expected API for the `assay-core::wizard` module and the two new MCP tools (`milestone_create`, `spec_create`). These tests will fail to compile after this task (the `wizard` module doesn't exist yet) — that's intentional and correct. The tests establish the contract that T02 and T04 must satisfy.

## Steps

1. Create `crates/assay-core/tests/wizard.rs` with a `use assay_core::wizard::{...}` import block (will cause compile error). Write 5 test functions using `tempfile::TempDir` for isolation:
   - `wizard_create_from_inputs_writes_files`: Build a `WizardInputs` with slug "my-feature", 2 chunks with 1 criterion each. Call `create_from_inputs(&inputs, assay_dir, specs_dir)`. Assert `milestone_load(assay_dir, "my-feature")` succeeds; assert both `specs_dir/my-feature-chunk-1/gates.toml` and `specs_dir/my-feature-chunk-2/gates.toml` exist and parse as `GatesSpec`.
   - `wizard_create_from_inputs_sets_milestone_and_order_on_specs`: Same setup; parse each generated `gates.toml`; assert `gates.milestone == Some("my-feature")` and `gates.order == Some(0)` / `Some(1)`.
   - `wizard_slug_collision_returns_error`: Call `create_from_inputs` twice with same inputs; assert second call returns `Err(...)`.
   - `wizard_create_spec_patches_milestone`: First create a milestone via `milestone_save`, then call `create_spec_from_params` with that milestone slug; reload the milestone and assert its `chunks` Vec contains the new chunk slug.
   - `wizard_create_spec_rejects_nonexistent_milestone`: Call `create_spec_from_params` with `milestone_slug = Some("ghost")` where no milestone file exists; assert it returns `Err`.

2. In `crates/assay-mcp/src/server.rs`, find the `#[cfg(test)]` block and add 5 test functions after the existing `cycle_*` tests:
   - `milestone_create_tool_in_router`: Assert `tool_names.contains(&"milestone_create")`.
   - `spec_create_tool_in_router`: Assert `tool_names.contains(&"spec_create")`.
   - `milestone_create_writes_milestone_toml`: Create temp project, call `server.milestone_create(...)` with slug "test-ms", name "Test MS", 1 chunk. Assert `!result.is_error`, parse the returned text as JSON string containing "test-ms". Assert `.assay/milestones/test-ms.toml` exists on disk.
   - `spec_create_writes_gates_toml`: Create temp project, call `server.spec_create(...)` with slug "chunk-1", name "Chunk 1". Assert `!result.is_error`. Assert `.assay/specs/chunk-1/gates.toml` exists on disk.
   - `spec_create_rejects_duplicate`: Call `spec_create` twice with same slug, assert second returns `is_error: true`.

3. Verify that `cargo test -p assay-core --features assay-types/orchestrate --test wizard` fails with a compile error mentioning `wizard`.

4. Verify that `cargo test -p assay-mcp -- milestone_create` fails with a compile error (params structs and methods don't exist yet).

5. Ensure test files compile cleanly once the wizard module and MCP tools are added — no logic errors in test assertions.

## Must-Haves

- [ ] `crates/assay-core/tests/wizard.rs` exists with 5 test functions
- [ ] All 5 tests import from `assay_core::wizard` (which doesn't exist — causes compile error)
- [ ] MCP test section has 5 new test functions referencing `MilestoneCreateParams`, `SpecCreateParams`
- [ ] Tests use `tempfile::TempDir` for isolation (no persistent disk state)
- [ ] `wizard_create_spec_patches_milestone` test sets up milestone via `assay_core::milestone::milestone_save` before calling `create_spec_from_params`

## Verification

- `cargo test -p assay-core --features assay-types/orchestrate --test wizard 2>&1 | grep "error\[E"` → compile errors present
- `cargo test -p assay-mcp -- milestone_create 2>&1 | grep "error\[E\|FAILED"` → fails
- After visual inspection: test assertions are logically correct and will pass once T02/T04 implement the module/tools

## Observability Impact

- Signals added/changed: None (tests only)
- How a future agent inspects this: Run `cargo test -p assay-core --features assay-types/orchestrate --test wizard` to see contract coverage
- Failure state exposed: Compile errors pinpoint exactly which types/functions are missing

## Inputs

- `crates/assay-core/src/milestone/mod.rs` — `milestone_save`, `milestone_load` signatures used in test setup
- `crates/assay-types/src/milestone.rs` — `Milestone`, `ChunkRef`, `MilestoneStatus` for test setup
- `crates/assay-mcp/src/server.rs` test section — `create_project()`, `extract_text()`, `AssayServer::new()` helpers already present
- S01-SUMMARY.md — established `milestone_save(assay_dir, &milestone)` signature

## Expected Output

- `crates/assay-core/tests/wizard.rs` — 5 failing tests defining the wizard core API contract
- `crates/assay-mcp/src/server.rs` — 5 new failing test functions in the `#[cfg(test)]` block
