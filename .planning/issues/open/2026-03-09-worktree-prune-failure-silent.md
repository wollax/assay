---
created: 2026-03-09T20:32
title: Git worktree prune failure silently discarded
area: core
provenance: github:wollax/assay#81
files:
  - crates/assay-core/src/worktree.rs:160
---

## Problem

In `list()`, `git worktree prune` failure is silently discarded via `let _ = git_command(...)`. If prune fails (e.g., read-only `.git` directory), stale worktrees will still appear in the list with no indication that pruning was skipped.

## Solution

Log a warning when prune fails. Don't fail the list operation, but make the skipped prune visible.
