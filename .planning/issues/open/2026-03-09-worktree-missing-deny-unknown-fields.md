---
created: 2026-03-09T21:15
title: WorktreeInfo and WorktreeStatus missing deny_unknown_fields
area: types
provenance: github:wollax/assay#89
files:
  - crates/assay-types/src/worktree.rs:31-32
  - crates/assay-types/src/worktree.rs:48-49
---

## Problem

`WorktreeInfo` and `WorktreeStatus` are missing `#[serde(deny_unknown_fields)]`, unlike `WorktreeConfig` and most other config/spec types in the codebase. Deserialization silently ignores typos or unexpected fields.

## Solution

Add `#[serde(deny_unknown_fields)]` to both types for consistency.
