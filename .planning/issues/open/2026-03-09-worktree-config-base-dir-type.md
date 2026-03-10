---
created: 2026-03-09T21:15
title: WorktreeConfig.base_dir uses String where Option<String> is idiomatic
area: types
provenance: github:wollax/assay#90
files:
  - crates/assay-types/src/worktree.rs:18
  - crates/assay-core/src/worktree.rs:116
---

## Problem

`base_dir` defaults to empty string via `#[serde(default)]`, and `resolve_worktree_dir` checks `.filter(|d| !d.is_empty())` to treat empty as absent. This "empty string means unset" pattern is fragile — a user setting `base_dir = ""` gets silent default behavior instead of a validation error.

## Solution

Use `Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]` to make "unset" explicit.
