---
estimated_steps: 7
estimated_files: 5
---

# T03: Register milestone_list and milestone_get MCP tools and add assay milestone CLI stub

**Slice:** S01 — Milestone & Chunk Type Foundation
**Milestone:** M005

## Description

Close the slice by wiring `assay-core::milestone` into two MCP tools (`milestone_list`, `milestone_get`) and a CLI stub (`assay milestone list`). The MCP tools must appear in the `AssayServer` tool router and return structured JSON. The CLI stub must run without error and print either a list of milestones or "No milestones found." This is the integration closure for S01 — every prior task's work becomes user-visible here.

MCP tool naming follows D067 (`milestone_` prefix). Both tools are additive (D005). No existing tool signatures are modified.

## Steps

1. In `crates/assay-mcp/src/server.rs`, add at the end of the parameter struct block:
   ```rust
   #[derive(Deserialize, JsonSchema)]
   pub struct MilestoneListParams {}
   
   #[derive(Deserialize, JsonSchema)]
   pub struct MilestoneGetParams {
       #[schemars(description = "Milestone slug (filename without .toml, e.g. 'my-feature')")]
       pub slug: String,
   }
   ```

2. Add `milestone_list` and `milestone_get` tool methods to the `AssayServer` impl block (before the `#[cfg(test)]` block). Both methods call `assay_core::milestone::milestone_scan` / `milestone_load` with `&self.assay_dir()`. On success, serialize the result to JSON string and return it as a text `CallToolResult`. On error, return `isError: true` with the error message. Method signatures:
   ```rust
   #[tool(description = "List all milestones in the current project. Returns an array of milestone summaries including slug, name, status, and chunk count.")]
   async fn milestone_list(&self, _params: Parameters<MilestoneListParams>) -> CallToolResult
   
   #[tool(description = "Get full details of a milestone by slug, including all chunk references and status.")]
   async fn milestone_get(&self, params: Parameters<MilestoneGetParams>) -> CallToolResult
   ```

3. Add `use assay_core::milestone::{milestone_load, milestone_scan};` to the imports in `server.rs`.

4. Add MCP tests (in `#[cfg(test)]` block) using the `serial` attribute like existing tests:
   - `milestone_list_tool_in_router` — asserts `"milestone_list"` appears in `server.tool_router.list_all()` names
   - `milestone_get_tool_in_router` — asserts `"milestone_get"` appears
   - `milestone_list_returns_empty_json_array_for_no_milestones` — calls `milestone_list` in a temp assay dir; asserts `!result.is_error` and response text parses as an empty JSON array `[]`
   - `milestone_get_returns_error_for_missing_slug` — calls `milestone_get` with slug `"nonexistent"`; asserts `result.is_error == true`

5. Create `crates/assay-cli/src/commands/milestone.rs`:
   ```rust
   use clap::Subcommand;
   // milestone_scan from assay_core::milestone
   // commands::assay_dir() for path
   
   #[derive(Subcommand)]
   pub enum MilestoneCommand {
       /// List all milestones in the current project
       List,
   }
   
   pub fn execute_milestone(cmd: MilestoneCommand) -> anyhow::Result<()> {
       match cmd {
           MilestoneCommand::List => milestone_list_cmd(),
       }
   }
   ```
   `milestone_list_cmd` calls `milestone_scan(&assay_dir())`, prints a table with columns `SLUG | NAME | STATUS` or prints `"No milestones found."` if the vec is empty. Print to stdout via `println!`.

6. In `crates/assay-cli/src/commands/mod.rs`, add `pub mod milestone;`.

7. In `crates/assay-cli/src/main.rs`:
   - Add `Milestone { #[command(subcommand)] command: commands::milestone::MilestoneCommand }` to the `Command` enum.
   - Add a match arm: `Command::Milestone { command } => commands::milestone::execute_milestone(command)?`.
   - Add a CLI doc comment for the subcommand.
   - Add a CLI test `milestone_list_subcommand_no_milestones` in the CLI test suite that creates a temp assay dir and runs `execute_milestone(MilestoneCommand::List)` directly, asserting `Ok(())`.

8. Run `cargo test -p assay-mcp -- milestone` and `cargo test -p assay-cli -- milestone` to confirm new tests pass. Run `just ready` to confirm full green.

## Must-Haves

- [ ] `milestone_list` appears in `AssayServer` tool router (checked by assertion test)
- [ ] `milestone_get` appears in `AssayServer` tool router (checked by assertion test)
- [ ] `milestone_list` returns a JSON array (empty `[]` for no milestones) with `is_error` false
- [ ] `milestone_get` returns `is_error: true` for a missing slug
- [ ] `assay milestone list` CLI subcommand compiles and runs without panicking in a project with no milestones (prints "No milestones found.")
- [ ] Total MCP tool count increases by 2 (from 22 to 24)
- [ ] `just ready` green — fmt + lint + test + deny all pass

## Verification

- `cargo test -p assay-mcp -- milestone` — all 4 new MCP tests pass
- `cargo test -p assay-cli -- milestone` — CLI test passes
- `cargo test -p assay-mcp -- tool_count` or `milestone_list_tool_in_router` confirms tool is present in router
- `just ready` exits 0

## Observability Impact

- Signals added/changed: `milestone_list` and `milestone_get` return `isError: true` with full error message on any I/O or parse failure; the error message includes the file path via `AssayError::Io`
- How a future agent inspects this: `mcp({ tool: "milestone_list" })` gives a live view of all milestones; `mcp({ tool: "milestone_get", args: '{"slug": "my-feature"}' })` shows full milestone detail; CLI `assay milestone list` gives human-readable output
- Failure state exposed: MCP tools surface `AssayError` messages directly to the agent as `isError: true` content — the agent can see exactly which slug failed and why

## Inputs

- `crates/assay-core/src/milestone/mod.rs` — `milestone_scan`, `milestone_load` (produced by T02)
- `crates/assay-types/src/milestone.rs` — `Milestone`, `MilestoneStatus` types (produced by T01)
- `crates/assay-mcp/src/server.rs` — existing `AssayServer` structure, tool registration pattern, test helpers (`create_project`, `extract_text`, `serial`)
- `crates/assay-cli/src/commands/spec.rs` — model for command module structure and `assay_dir()` usage
- `crates/assay-cli/src/main.rs` — existing `Command` enum to extend
- `.kata/DECISIONS.md` D005 (additive MCP only), D067 (milestone_ prefix convention)

## Expected Output

- `crates/assay-mcp/src/server.rs` — `MilestoneListParams`, `MilestoneGetParams`, `milestone_list`, `milestone_get` methods, 4 new tests
- `crates/assay-cli/src/commands/milestone.rs` — new file with `MilestoneCommand`, `execute_milestone`, `milestone_list_cmd`
- `crates/assay-cli/src/commands/mod.rs` — `pub mod milestone` added
- `crates/assay-cli/src/main.rs` — `Milestone` variant in `Command` enum + dispatch arm
