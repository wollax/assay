---
created: 2026-03-09T20:32
title: Cleanup --all should use canonical path from git
area: cli
provenance: github:wollax/assay#79
files:
  - crates/assay-cli/src/commands/worktree.rs:382-389
---

## Problem

`handle_worktree_cleanup_all` computes worktree path as `worktree_dir.join(&entry.spec_slug)`, then falls back to `entry.path` if the computed path doesn't exist. This heuristic can silently use a stale `entry.path` from `git worktree list` that may differ from what the user expects based on `--worktree-dir`.

## Solution

Always use `entry.path` (the canonical path from git) for cleanup operations, since `git worktree list` already returns the actual path.
