---
phase: 28
plan: 2
type: execute
wave: 2
depends_on: [1]
files_modified:
  - crates/assay-cli/src/main.rs
  - crates/assay-cli/src/commands/mod.rs
  - crates/assay-cli/src/commands/worktree.rs
  - crates/assay-mcp/src/server.rs
autonomous: true
must_haves:
  truths:
    - "assay worktree create <spec> creates a worktree and prints path"
    - "assay worktree list shows a compact table of all assay worktrees"
    - "assay worktree status <spec> reports branch, dirty state, ahead/behind"
    - "assay worktree cleanup <spec> removes worktree and branch with confirmation for dirty state"
    - "assay worktree cleanup --all removes all worktrees with confirmation"
    - "--force flag bypasses confirmation prompts"
    - "All subcommands support --json for machine-readable output"
    - "--worktree-dir flag overrides worktree base directory"
    - "MCP tools worktree_create, worktree_list, worktree_status, worktree_cleanup are callable"
    - "MCP tools return structured JSON results via CallToolResult"
    - "Non-interactive mode (no TTY) fails safely for destructive operations without --force"
  artifacts:
    - path: "crates/assay-cli/src/commands/worktree.rs"
      provides: "WorktreeCommand enum with Create/List/Status/Cleanup variants and handle() dispatcher"
    - path: "crates/assay-cli/src/main.rs"
      provides: "Worktree variant in Command enum wired to commands::worktree"
    - path: "crates/assay-mcp/src/server.rs"
      provides: "4 MCP tools: worktree_create, worktree_list, worktree_status, worktree_cleanup"
  key_links:
    - from: "commands/worktree.rs handle()"
      to: "assay_core::worktree::{create,list,status,cleanup}"
      via: "CLI delegates all logic to core module"
    - from: "server.rs worktree_create tool"
      to: "assay_core::worktree::create"
      via: "MCP tools delegate to same core functions as CLI"
    - from: "main.rs Command::Worktree"
      to: "commands::worktree::handle()"
      via: "Standard dispatch pattern: match arm calls handle()"
---

<objective>
Wire the CLI subcommands and MCP tools for worktree management, consuming the core module from Plan 01. This provides human-facing CLI output (table/JSON) and agent-facing MCP tools for all four operations.

Purpose: ORCH-01 through ORCH-06 (CLI surface), ORCH-05 (MCP surface) — complete user-facing and agent-facing interfaces.
Output: commands/worktree.rs with 4 subcommands, 4 MCP tools on AssayServer, wiring in main.rs.
</objective>

<execution_context>
<!-- Executor agent has built-in instructions -->
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/ROADMAP.md
@.planning/STATE.md
@.planning/phases/pending/28-worktree-manager/28-CONTEXT.md
@.planning/phases/pending/28-worktree-manager/28-RESEARCH.md
@crates/assay-cli/src/main.rs
@crates/assay-cli/src/commands/mod.rs
@crates/assay-cli/src/commands/checkpoint.rs
@crates/assay-mcp/src/server.rs
@crates/assay-core/src/worktree.rs
</context>

<tasks>
<task type="auto">
  <name>Task 1: CLI worktree subcommands</name>
  <files>
    - crates/assay-cli/src/commands/worktree.rs
    - crates/assay-cli/src/commands/mod.rs
    - crates/assay-cli/src/main.rs
  </files>
  <action>
    1. Create `crates/assay-cli/src/commands/worktree.rs` following the pattern in checkpoint.rs:

       **WorktreeCommand enum** (clap Subcommand derive):
       - `Create { name: String, #[arg(long)] base: Option<String>, #[arg(long)] worktree_dir: Option<String>, #[arg(long)] json: bool }`
       - `List { #[arg(long)] json: bool, #[arg(long)] worktree_dir: Option<String> }`
       - `Status { name: String, #[arg(long)] json: bool, #[arg(long)] worktree_dir: Option<String> }`
       - `Cleanup { name: Option<String>, #[arg(long)] all: bool, #[arg(long)] force: bool, #[arg(long)] json: bool, #[arg(long)] worktree_dir: Option<String> }`

       **pub(crate) fn handle(command: WorktreeCommand) -> anyhow::Result<i32>** — dispatch to handlers below.

       **handle_worktree_create**:
       - Resolve project root, load config (`assay_core::config::load`)
       - Resolve worktree dir via `assay_core::worktree::resolve_worktree_dir(cli_override, &config, &root)`
       - Resolve specs dir from config
       - Call `assay_core::worktree::create(root, name, base, worktree_dir, specs_dir)`
       - Human output: "Created worktree for '{name}' at {path}\n  Branch: {branch}\n  Base: {base_branch}"
       - JSON output: serialize WorktreeInfo

       **handle_worktree_list**:
       - Resolve project root, config, worktree dir
       - Call `assay_core::worktree::list(root, worktree_dir)`
       - Human output: compact table with columns: Spec, Branch, Path, Status (use colors_enabled())
         - For each entry, optionally call status to get dirty state (or just show path/branch from WorktreeInfo)
       - JSON output: serialize Vec<WorktreeInfo>
       - Empty state: "No active worktrees."

       **handle_worktree_status**:
       - Resolve project root, config, worktree dir
       - Compute worktree_path = worktree_dir.join(name)
       - Call `assay_core::worktree::status(worktree_path, name)`
       - Human output: "Worktree: {spec_slug}\n  Branch: {branch}\n  HEAD: {head}\n  Status: {clean|dirty}\n  Ahead: {ahead}  Behind: {behind}"
       - JSON output: serialize WorktreeStatus

       **handle_worktree_cleanup**:
       - If `--all` flag: call list, then iterate and cleanup each. Confirm interactively (list all names, ask y/N) unless `--force`.
       - If specific name: compute worktree_path, check dirty state, confirm if dirty and interactive and not --force.
       - TTY detection: `use std::io::IsTerminal; std::io::stdin().is_terminal()`
       - Non-interactive + dirty + no --force = error message and exit 1
       - Call `assay_core::worktree::cleanup(root, worktree_path, spec_slug, force)`
       - Human output: "Removed worktree for '{name}'"
       - JSON output: `{"removed": "name"}`
       - Require either name or --all (error if neither)

    2. In `crates/assay-cli/src/commands/mod.rs`:
       - Add `pub mod worktree;` declaration

    3. In `crates/assay-cli/src/main.rs`:
       - Add Worktree variant to Command enum:
         ```rust
         /// Manage git worktrees for spec isolation
         #[command(after_long_help = "...")]
         Worktree {
             #[command(subcommand)]
             command: commands::worktree::WorktreeCommand,
         },
         ```
       - Add match arm in run():
         ```rust
         Some(Command::Worktree { command }) => commands::worktree::handle(command),
         ```
       - Add help text examples covering create, list, status, cleanup
  </action>
  <verify>
    cargo build -p assay-cli 2>&1 | tail -5
    cargo run -p assay-cli -- worktree --help 2>&1
    cargo run -p assay-cli -- worktree create --help 2>&1
  </verify>
  <done>
    - `assay worktree --help` shows all 4 subcommands
    - `assay worktree create --help` shows name, --base, --worktree-dir, --json flags
    - `assay worktree cleanup --help` shows name, --all, --force, --json flags
    - Human output uses compact table for list, structured output for status
    - --json flag on all subcommands produces valid JSON
    - Non-interactive cleanup of dirty worktree without --force fails with actionable error
  </done>
</task>

<task type="auto">
  <name>Task 2: MCP worktree tools</name>
  <files>
    - crates/assay-mcp/src/server.rs
  </files>
  <action>
    1. Add parameter structs for the 4 worktree tools, following existing pattern (SpecGetParams, GateRunParams):

       **WorktreeCreateParams**:
       - `name: String` — spec name (slug)
       - `base: Option<String>` — base branch override
       - `worktree_dir: Option<String>` — worktree directory override

       **WorktreeListParams**:
       - `worktree_dir: Option<String>` — worktree directory override

       **WorktreeStatusParams**:
       - `name: String` — spec name (slug)
       - `worktree_dir: Option<String>` — worktree directory override

       **WorktreeCleanupParams**:
       - `name: String` — spec name (slug)
       - `force: Option<bool>` — force cleanup of dirty worktrees (default: true for MCP since non-interactive)
       - `worktree_dir: Option<String>` — worktree directory override

    2. Add 4 `#[tool]` methods to AssayServer impl:

       **worktree_create** — "Create an isolated git worktree for a spec":
       - Resolve project root (same pattern as existing tools)
       - Load config, resolve worktree dir and specs dir
       - Call `assay_core::worktree::create(...)`
       - Return WorktreeInfo as JSON in CallToolResult content
       - On error: return domain error as CallToolResult with isError: true

       **worktree_list** — "List all active assay worktrees":
       - Resolve project root, config, worktree dir
       - Call `assay_core::worktree::list(...)`
       - Return Vec<WorktreeInfo> as JSON

       **worktree_status** — "Check worktree status (branch, dirty, ahead/behind)":
       - Resolve project root, config, worktree dir
       - Compute worktree path
       - Call `assay_core::worktree::status(...)`
       - Return WorktreeStatus as JSON

       **worktree_cleanup** — "Remove a worktree and its branch":
       - Resolve project root, config, worktree dir
       - Compute worktree path
       - Default force=true for MCP (agents are non-interactive)
       - Call `assay_core::worktree::cleanup(...)`
       - Return success message as JSON

    3. Register the 4 new tools in the `#[tool_router]` macro (or however tools are registered — follow existing pattern).

    4. Update the module-level doc comment to mention the new tools.
  </action>
  <verify>
    cargo check -p assay-mcp 2>&1 | tail -5
    cargo test -p assay-mcp 2>&1 | tail -20
    just lint 2>&1 | tail -10
  </verify>
  <done>
    - 4 MCP tools compile (worktree_create, worktree_list, worktree_status, worktree_cleanup)
    - Parameter structs derive Deserialize + JsonSchema with schemars descriptions
    - Tools follow existing error handling pattern (domain errors as CallToolResult with isError)
    - MCP cleanup defaults force=true (non-interactive agent context)
    - `just ready` passes
  </done>
</task>
</tasks>

<verification>
```bash
just ready
cargo run -p assay-cli -- worktree --help
```
</verification>

<success_criteria>
- [ ] `assay worktree create <spec>` creates worktree and prints human/JSON output
- [ ] `assay worktree list` shows compact table (or JSON)
- [ ] `assay worktree status <spec>` reports branch, dirty, ahead/behind
- [ ] `assay worktree cleanup <spec>` removes worktree with confirmation logic
- [ ] `assay worktree cleanup --all` removes all worktrees with bulk confirmation
- [ ] `--force` bypasses all confirmation prompts
- [ ] `--json` works on all 4 subcommands
- [ ] `--worktree-dir` override works on all 4 subcommands
- [ ] 4 MCP tools registered and callable with correct parameter schemas
- [ ] MCP cleanup defaults to force=true (non-interactive)
- [ ] `just ready` passes
</success_criteria>

<output>
After completion, create `.planning/phases/28-worktree-manager/28-02-SUMMARY.md`
</output>
