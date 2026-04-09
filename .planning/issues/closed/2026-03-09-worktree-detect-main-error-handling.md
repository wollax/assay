---
created: 2026-03-09T21:15
title: detect_main_worktree conflates errors with "is main worktree"
area: core
provenance: github:wollax/assay#88
files:
  - crates/assay-core/src/worktree.rs:296-314
---

## Problem

`detect_main_worktree` returns `Option<PathBuf>` where `None` means either "this is the main worktree" or "an I/O error occurred." `read_to_string(&dot_git).ok()?` converts permission errors into `None`. The function also assumes the `.git/worktrees/<name>` directory layout which may differ with `core.worktreeDir` or `GIT_COMMON_DIR` overrides.

## Solution

Return `Result<Option<PathBuf>>` to distinguish errors from the "is main worktree" case. Consider using `git rev-parse --show-toplevel` for more robust repo root detection.
