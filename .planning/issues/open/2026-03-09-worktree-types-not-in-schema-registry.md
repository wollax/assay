---
created: 2026-03-09T21:15
title: WorktreeInfo and WorktreeStatus not registered in schema registry
area: types
provenance: github:wollax/assay#91
files:
  - crates/assay-types/src/worktree.rs:31-43
  - crates/assay-types/src/worktree.rs:48-64
---

## Problem

`WorktreeInfo` and `WorktreeStatus` derive `JsonSchema` but are not submitted to the schema registry via `inventory::submit!`, unlike `WorktreeConfig`. If the registry is used for documentation or validation tooling, these types will be absent.

## Solution

Add `inventory::submit!` entries for both types, or document why they are intentionally excluded.
