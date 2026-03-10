---
created: 2026-03-09T20:32
title: MCP worktree_list skips project validation
area: mcp
provenance: github:wollax/assay#86
files:
  - crates/assay-mcp/src/server.rs:818
---

## Problem

`worktree_list` MCP tool doesn't call `load_config`, so it doesn't validate that an Assay project exists before listing worktrees. The other three worktree MCP tools do check config. This means `worktree_list` could show unrelated worktrees in a non-Assay directory or fail with a generic git error.

## Solution

Add `load_config` call to `worktree_list` for consistency with other worktree MCP tools, even if config values aren't needed for the operation itself.
