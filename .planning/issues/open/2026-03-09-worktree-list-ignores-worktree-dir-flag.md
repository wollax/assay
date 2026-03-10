---
created: 2026-03-09T21:15
title: worktree list --worktree-dir flag is accepted but ignored
area: cli
provenance: github:wollax/assay#92
files:
  - crates/assay-cli/src/commands/worktree.rs:44-51
  - crates/assay-cli/src/commands/worktree.rs:176
---

## Problem

The `list` subcommand accepts `--worktree-dir` but the resolved directory is unused (prefixed with `_`). The core `list` function discovers worktrees from git, not the filesystem. The flag is silently ignored, confusing users who expect it to narrow scope.

## Solution

Either remove `--worktree-dir` from the `List` variant, or document it as reserved for future use.
