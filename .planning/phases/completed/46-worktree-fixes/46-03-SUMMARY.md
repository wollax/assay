---
phase: 46-worktree-fixes
plan: 03
subsystem: worktree
tags: [warning-surfacing, prune, mcp, wrapper-type]
depends_on: [46-02]
files_modified:
  - crates/assay-core/src/worktree.rs
  - crates/assay-cli/src/commands/worktree.rs
  - crates/assay-mcp/src/server.rs
decisions:
  - "WorktreeListResult lives in assay-core, not assay-types — no Serialize/Deserialize derives"
  - "CLI destructures and ignores warnings — no display change for CLI users"
  - "MCP WorktreeListResponse uses skip_serializing_if for clean output when no warnings"
metrics:
  tasks_completed: 2
  tasks_total: 2
  deviations: 0
  duration: "~4 min"
---

# Phase 46 Plan 03: Prune Failure Warning Surfacing Summary

## Objective

Change `list()` to return a `WorktreeListResult` wrapper struct that surfaces prune failures as warnings instead of silently discarding them. Update all three callsites (2 CLI, 1 MCP).

## What Changed

### Task 1: Add WorktreeListResult and update list()

Defined `WorktreeListResult { entries: Vec<WorktreeInfo>, warnings: Vec<String> }` in `crates/assay-core/src/worktree.rs` as an internal type (no serde derives).

Changed `list()` return type from `Result<Vec<WorktreeInfo>>` to `Result<WorktreeListResult>`. Replaced `let _ = git_command(&["worktree", "prune"], ...)` with warning capture via `if let Err(e) = ...`.

Updated two `list()` calls in integration tests to append `.entries`.

### Task 2: Update CLI and MCP callsites

**CLI** (`crates/assay-cli/src/commands/worktree.rs`):
- `handle_worktree_list`: destructures `result.entries`, serializes entries only
- `handle_worktree_cleanup_all`: destructures `result.entries`, warnings ignored

**MCP** (`crates/assay-mcp/src/server.rs`):
- Added `WorktreeListResponse` struct with `#[serde(skip_serializing_if = "Vec::is_empty")]` on `warnings`
- Handler wraps `result.entries` and `result.warnings` into the response struct before serialization

## Verification

- `cargo test --workspace`: 840 passed, 3 ignored
- `just ready`: all checks passed (fmt, lint, test, deny)

## Commits

| Hash | Message |
|------|---------|
| e00e27b | refactor(46-03): return WorktreeListResult from list() with prune warnings |
| 8289331 | feat(46-03): propagate prune warnings through CLI and MCP callsites |

## Deviations

None. Plan executed as written.
