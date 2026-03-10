---
created: 2026-03-09T21:15
title: Missing test for resolve_worktree_dir with empty base_dir config
area: core
provenance: github:wollax/assay#95
files:
  - crates/assay-core/src/worktree.rs:116
---

## Problem

`resolve_worktree_dir` has an explicit `.filter(|d| !d.is_empty())` guard that treats `WorktreeConfig { base_dir: "" }` the same as `None`. No test verifies this fallthrough behavior.

## Solution

Add a unit test with `make_config(Some(""))` and verify it falls through to the default `../<project>-worktrees/` path.
