---
created: 2026-03-09T20:32
title: Worktree status check silently treats errors as clean
area: cli
provenance: github:wollax/assay#78
files:
  - crates/assay-cli/src/commands/worktree.rs:199-206
---

## Problem

In `handle_worktree_cleanup`, the dirty state check uses `unwrap_or(false)` on the `status()` call result. If status fails for a reason other than "not found" (e.g., corrupted git state), it silently treats the worktree as clean, potentially skipping the interactive confirmation prompt and deleting a worktree with uncommitted changes.

## Solution

Distinguish between "not found" errors (treat as proceed) and other errors (propagate or warn). Consider matching on error variant to handle `WorktreeNotFound` specifically.
