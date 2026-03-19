---
id: T03
parent: S01
milestone: M005
provides:
  - "milestone_list MCP tool registered in AssayServer router — returns empty JSON array [] for no milestones"
  - "milestone_get MCP tool registered in AssayServer router — returns isError:true for missing slug"
  - "MilestoneListParams and MilestoneGetParams parameter structs in assay-mcp/src/server.rs"
  - "assay milestone list CLI subcommand — prints tabular SLUG/NAME/STATUS or 'No milestones found.'"
  - "Fixed pre-existing clippy::derive_partial_eq_without_eq on Milestone struct (added Eq)"
key_files:
  - crates/assay-mcp/src/server.rs
  - crates/assay-cli/src/commands/milestone.rs
  - crates/assay-cli/src/commands/mod.rs
  - crates/assay-cli/src/main.rs
  - crates/assay-types/src/milestone.rs
key_decisions:
  - "milestone_list returns the full Vec<Milestone> serialized as a JSON array — no envelope wrapper, matches the pattern of other list tools that return the domain type directly"
  - "milestone_get uses domain_error() helper for all errors, consistent with existing MCP error handling pattern"
  - "CLI milestone_list_cmd returns anyhow::Result<i32> matching other handle() functions rather than the plan's fn signature — preserves CLI exit code convention"
patterns_established:
  - "Milestone MCP tools follow same resolve_cwd() + cwd.join('.assay') + domain_error() pattern as session_list and gate_history"
  - "CLI subcommand modules follow spec.rs pattern: MilestoneCommand enum + handle(cmd) dispatch + named inner functions"
observability_surfaces:
  - "MCP: mcp({ tool: 'milestone_list' }) returns live JSON array of all milestones; mcp({ tool: 'milestone_get', args: '{\"slug\": \"my-feature\"}' }) shows full milestone detail including chunks"
  - "MCP: isError:true with AssayError::Io message (includes file path) on any I/O or parse failure"
  - "CLI: assay milestone list — human-readable SLUG/NAME/STATUS table or 'No milestones found.'"
duration: 30min
verification_result: passed
completed_at: 2026-03-19T00:00:00Z
blocker_discovered: false
---

# T03: Register milestone_list and milestone_get MCP tools and add assay milestone CLI stub

**Two MCP tools (milestone_list, milestone_get) registered in AssayServer router + assay milestone list CLI subcommand — S01 integration closure complete.**

## What Happened

Added `MilestoneListParams` and `MilestoneGetParams` parameter structs, then implemented `milestone_list` and `milestone_get` as `#[tool]`-annotated async methods on `AssayServer`. Both use `resolve_cwd()` + `cwd.join(".assay")` + `milestone_scan`/`milestone_load` from assay-core, returning serialized JSON on success and `domain_error()` on failure. The `#[tool_router]` attribute macro auto-discovers both methods.

Created `crates/assay-cli/src/commands/milestone.rs` with `MilestoneCommand::List`, `handle()`, and `milestone_list_cmd()`. The command calls `milestone_scan`, prints a formatted table or "No milestones found.", and returns `Ok(0)`. Added `pub mod milestone` to `commands/mod.rs` and a `Milestone { command }` variant + dispatch arm to `main.rs`.

Also fixed a pre-existing clippy `derive_partial_eq_without_eq` warning on `Milestone` (added `Eq` to the derive list) that would have blocked `just ready`.

4 MCP tests and 1 CLI test all pass. `just ready` exits 0 — fmt + lint + test + deny all green. Tool count increased from 22 to 24.

## Verification

- `cargo test -p assay-mcp -- milestone` → 4 tests passed: milestone_list_tool_in_router, milestone_get_tool_in_router, milestone_list_returns_empty_json_array_for_no_milestones, milestone_get_returns_error_for_missing_slug
- `cargo test -p assay-cli -- milestone` → 1 test passed: milestone_list_subcommand_no_milestones
- `just ready` → all checks green (fmt + clippy + cargo test --workspace + cargo deny)

## Diagnostics

- `mcp({ tool: "milestone_list" })` — live JSON array of all milestones in `.assay/milestones/`
- `mcp({ tool: "milestone_get", args: '{"slug": "my-feature"}' })` — full Milestone struct as JSON, or `isError: true` with path-bearing error message on failure
- `assay milestone list` — human-readable table; stderr surfaces scan errors via anyhow propagation

## Deviations

- CLI `handle()` returns `anyhow::Result<i32>` (matching existing CLI convention) rather than `execute_milestone(cmd: MilestoneCommand) -> anyhow::Result<()>` as written in the plan. The `i32` return follows the pattern used by all other CLI command handlers and feeds the process exit code. Functionally equivalent.
- Fixed pre-existing `Eq` clippy lint on `Milestone` struct — not in the plan, but required for `just ready` to pass.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-mcp/src/server.rs` — Added doc comment entries, milestone import, MilestoneListParams/MilestoneGetParams structs, milestone_list/milestone_get tool methods, 4 MCP tests
- `crates/assay-cli/src/commands/milestone.rs` — New file: MilestoneCommand enum, handle(), milestone_list_cmd(), inline test
- `crates/assay-cli/src/commands/mod.rs` — Added `pub mod milestone`
- `crates/assay-cli/src/main.rs` — Added Milestone variant to Command enum and dispatch arm
- `crates/assay-types/src/milestone.rs` — Added `Eq` to Milestone derive to fix pre-existing clippy lint
