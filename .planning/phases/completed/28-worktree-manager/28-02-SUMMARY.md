# Phase 28 Plan 02: CLI Subcommands and MCP Tools Summary

Wired 4 CLI subcommands and 4 MCP tools for worktree lifecycle management, consuming the core module from Plan 01.

## Tasks

| # | Task | Status |
|---|------|--------|
| 1 | CLI worktree subcommands | Done |
| 2 | MCP worktree tools | Done |

## Commits

- `5fd5a01`: feat(28-02): add CLI worktree subcommands
- `0249420`: feat(28-02): add MCP worktree tools

## What Was Built

### CLI (`crates/assay-cli/src/commands/worktree.rs`)

- `assay worktree create <spec>` — creates worktree, prints human or JSON output
- `assay worktree list` — compact table of all assay worktrees (or JSON)
- `assay worktree status <spec>` — branch, HEAD, dirty state, ahead/behind
- `assay worktree cleanup <spec|--all>` — removes worktree + branch with interactive confirmation for dirty state
- All subcommands support `--json` and `--worktree-dir` flags
- `--force` bypasses confirmation prompts
- Non-interactive mode (no TTY) fails safely for destructive operations without `--force`
- Wired into `main.rs` Command enum and `commands/mod.rs`

### MCP (`crates/assay-mcp/src/server.rs`)

- `worktree_create` — creates worktree, returns WorktreeInfo JSON
- `worktree_list` — returns array of WorktreeInfo
- `worktree_status` — returns WorktreeStatus JSON
- `worktree_cleanup` — removes worktree, defaults force=true (non-interactive agent context)
- All tools follow existing error pattern (domain errors as CallToolResult with isError)
- Parameter structs derive Deserialize + JsonSchema with schemars descriptions

## Deviations

- **CLI integration smoke tests deferred**: Plan called for `assert_cmd`-based integration tests, but `assert_cmd` is not a workspace dependency and adding it would violate the zero-new-dependency constraint. Core module has comprehensive integration tests covering the same operations. Tracked in `.planning/issues/open/2026-03-09-cli-integration-tests.md`.
- **ORCH-07 worktree-aware spec resolution not wired**: The plan's objective mentioned wiring `detect_main_worktree` into CLI/MCP for auto-resolving specs from main repo. This was explicitly scoped as "if feasible" and deferred — the detection function exists in core but the CLI currently resolves specs from CWD, which works correctly for worktrees (they have their own checkout of the spec files). Full worktree-aware resolution requires changes to the config/spec resolution path and belongs in a follow-up.
- **`check-plugin-version` failure is pre-existing**: Plugin version mismatch (0.1.0 vs 0.2.0) predates this plan. All other `just ready` targets pass (fmt-check, lint, test, deny).

## Verification

- `just fmt-check` — pass
- `just lint` — pass (clippy clean)
- `just test` — pass (all 61 MCP tests, all core worktree tests)
- `just deny` — pass
- `assay worktree --help` — shows all 4 subcommands
- `assay worktree create --help` — shows name, --base, --worktree-dir, --json
- `assay worktree cleanup --help` — shows name, --all, --force, --json

## Duration

~7 minutes
