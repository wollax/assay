---
created: 2026-03-16T15:30
title: git_command helper duplicated between worktree.rs and merge.rs
area: core
provenance: local
files:
  - crates/assay-core/src/worktree.rs:32-56
  - crates/assay-core/src/merge.rs:41-65
---

## Problem

`git_command(args, cwd) -> Result<String>` is byte-for-byte identical in `worktree.rs` and `merge.rs`. `git_raw` in `merge.rs` is a new variant that returns (stdout, stderr, exit_code). Both use `WorktreeGit`/`WorktreeGitFailed` error variants despite not being worktree-specific.

## Solution

Extract to a shared `crate::git` module (e.g., `crates/assay-core/src/git.rs`) with both `git_command` and `git_raw`. Rename error variants from `WorktreeGit`/`WorktreeGitFailed` to `GitSpawnFailed`/`GitCommandFailed`. Both `worktree` and `merge` import from the shared module.
