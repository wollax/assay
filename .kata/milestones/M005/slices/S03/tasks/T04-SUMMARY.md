---
id: T04
parent: S03
milestone: M005
provides:
  - "MCP `milestone_create` tool registered in AssayServer router"
  - "MCP `spec_create` tool registered in AssayServer router"
  - "`MilestoneChunkInput`, `MilestoneCreateParams`, `SpecCreateParams` param structs"
  - "`create_spec_from_params` updated to accept `criteria: Vec<String>` parameter"
  - "5 MCP wizard tests passing: milestone_create_tool_in_router, spec_create_tool_in_router, milestone_create_writes_milestone_toml, spec_create_writes_gates_toml, spec_create_rejects_duplicate"
key_files:
  - "crates/assay-mcp/src/server.rs"
  - "crates/assay-core/src/wizard.rs"
  - "crates/assay-core/tests/wizard.rs"
key_decisions:
  - "Tests use `MilestoneChunkInput` (not `ChunkParams`) and `criteria: Vec<String>` (not `Vec<CriterionParams>`) — test shapes override the plan's suggested struct names; tests are the authoritative contract"
  - "`create_spec_from_params` gained a `criteria: Vec<String>` parameter (appended last) so the MCP tool passes real criteria through to `write_gates_toml`; existing callers updated to pass `vec![]`"
  - "`milestone_create` converts `MilestoneChunkInput` chunks to `(String, u32)` tuples via `enumerate()` — order is positional, not caller-supplied, matching the test shape that has no `order` field on chunks"
  - "`SpecCreateParams` includes `description: Option<String>` (accepted but not forwarded to `create_spec_from_params` since that function ignores `_name`) — present in tests, kept for forward compatibility"
patterns_established:
  - "`milestone_create` and `spec_create` use `spawn_blocking` wrapping identical to `cycle_advance` — blocking wizard I/O must not run on the async executor"
  - "MCP tool param structs derive only `Deserialize, JsonSchema` (not `Debug`) — same as `MilestoneListParams`/`MilestoneGetParams` pattern; `Debug` not required for non-debug tools"
observability_surfaces:
  - "`milestone_create` success → JSON-encoded slug string; `isError: true` + AssayError::Io message (\"milestone '<slug>' already exists\") on collision"
  - "`spec_create` success → JSON-encoded absolute path to created gates.toml; `isError: true` + AssayError::Io message (\"spec directory '<slug>' already exists\") on duplicate"
  - "Both tools return `domain_error(&e)` on all AssayError variants — failure is externally inspectable, never swallowed"
duration: 25m
verification_result: passed
completed_at: 2026-03-20
blocker_discovered: false
---

# T04: Implement `milestone_create` and `spec_create` MCP Tools

**Added `milestone_create` and `spec_create` tools to `AssayServer`; all 5 MCP wizard tests pass and `just ready` is green with zero regressions.**

## What Happened

Added the two MCP authoring tools that close S03's programmatic entry point for agent callers. Key changes:

1. **`crates/assay-core/src/wizard.rs`** — `create_spec_from_params` gained a `criteria: Vec<String>` trailing parameter so the MCP tool can forward user-supplied criteria into the generated `gates.toml`. The two existing callers in `crates/assay-core/tests/wizard.rs` were updated to pass `vec![]`.

2. **`crates/assay-mcp/src/server.rs`** — Added three param structs (`MilestoneChunkInput`, `MilestoneCreateParams`, `SpecCreateParams`) near the existing `ChunkStatusParams` block. Added `milestone_create()` and `spec_create()` methods with `#[tool(...)]` attribute annotations and `spawn_blocking` wrapping. Both methods use `resolve_cwd()`, `load_config()`, and `domain_error(&e)` following the `cycle_advance` pattern. Updated the module-level doc comment to list the two new tools.

The struct names diverge from the task plan (`MilestoneChunkInput` vs `ChunkParams`, no `CriterionParams`) because the tests written in T01 are the authoritative contract and they used these names.

## Verification

```
cargo test -p assay-mcp -- milestone_create spec_create
# 5 passed, 0 failed

cargo test --workspace
# all test results: ok, 0 failed (1320+ tests across all crates)

just ready
# fmt-check, lint, tests, deny — all green
```

Slice-level verification:
- ✅ `cargo test -p assay-core --features assay-types/orchestrate --test wizard` — 5 tests pass (T02 deliverable, confirmed still green)
- ✅ `cargo test -p assay-mcp -- milestone_create` — 3 tests pass
- ✅ `cargo test -p assay-mcp -- spec_create` — 3 tests pass (includes `spec_create_rejects_duplicate`)
- ✅ `cargo test -p assay-cli -- plan` — passes (T03 deliverable, confirmed still green)
- ✅ `cargo test --workspace` — green
- ✅ `just ready` — green

## Diagnostics

- `milestone_create` success: response text is a JSON string of the slug (e.g. `"test-ms"`); usable directly in subsequent `cycle_status` or `milestone_get` calls
- `milestone_create` failure (collision): `isError: true`, text contains `"milestone 'test-ms' already exists"`
- `spec_create` success: response text is a JSON string of the absolute path to `gates.toml`
- `spec_create` failure (duplicate): `isError: true`, text contains `"spec directory 'dup-chunk' already exists"`
- `spec_create` failure (bad milestone slug): `isError: true`, propagates `milestone_load` error

## Deviations

- **Struct naming**: Plan proposed `ChunkParams` with an `order: u32` field; tests use `MilestoneChunkInput` with no `order` field (order is derived positionally via `enumerate()`). Tests are authoritative — plan's struct names were not used.
- **No `CriterionParams`**: Plan proposed `CriterionParams { name, description, cmd }` and converting to `Vec<CriterionInput>`. Tests use `criteria: Vec<String>` directly. Implemented `criteria: Vec<String>` in `SpecCreateParams` and updated `create_spec_from_params` to accept `Vec<String>` rather than adding a `CriterionParams` conversion layer.
- **`create_spec_from_params` signature change**: Added `criteria: Vec<String>` as a new trailing parameter. Two existing test callers updated to pass `vec![]`. This is the only cross-crate change outside the plan scope.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-mcp/src/server.rs` — added `MilestoneChunkInput`, `MilestoneCreateParams`, `SpecCreateParams` structs; added `milestone_create()` and `spec_create()` `#[tool]` methods; updated module doc comment
- `crates/assay-core/src/wizard.rs` — added `criteria: Vec<String>` parameter to `create_spec_from_params`; updated function body to pass criteria to `write_gates_toml`
- `crates/assay-core/tests/wizard.rs` — updated two `create_spec_from_params` call sites to pass `vec![]` for the new criteria parameter
