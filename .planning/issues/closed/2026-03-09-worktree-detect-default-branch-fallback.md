---
created: 2026-03-09T20:32
title: Default branch fallback to "main" gives confusing errors
area: core
provenance: github:wollax/assay#82
files:
  - crates/assay-core/src/worktree.rs:154
---

## Problem

`detect_default_branch` silently falls back to "main" on any failure. If the repo uses "master" as its default branch, `git worktree add` will fail with a confusing `WorktreeGitFailed` error about a missing ref rather than a clear "could not detect default branch" message.

## Solution

Consider checking if the detected/fallback branch actually exists before using it, or provide a clearer error message when the worktree add fails due to a missing base branch ref.
