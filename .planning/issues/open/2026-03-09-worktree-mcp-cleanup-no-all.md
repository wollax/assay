---
created: 2026-03-09T21:15
title: MCP worktree_cleanup tool has no --all equivalent
area: mcp
provenance: github:wollax/assay#94
files:
  - crates/assay-mcp/src/server.rs:1009
---

## Problem

The CLI supports `--all` for batch cleanup, but the MCP tool requires a specific `name`. An agent wanting to clean up all worktrees must call `worktree_list` then loop over `worktree_cleanup` for each, which is chattier and more error-prone.

## Solution

Add an optional `all: bool` parameter to `WorktreeCleanupParams`, or create a separate `worktree_cleanup_all` tool.
