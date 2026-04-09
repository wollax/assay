---
created: 2026-03-16T13:45
title: Missing test for list() returning non-empty warnings
area: testing
provenance: github:wollax/assay#143
files:
  - crates/assay-core/src/worktree.rs:325-337
---

## Problem

`WorktreeListResult.warnings` field is never exercised in any test. The `list()` function captures `git worktree prune` failures as warnings, but no test verifies this behavior. Hard to trigger in integration tests since prune rarely fails in clean temp repos.

## Solution

Add a unit-level test that verifies warning population when prune fails. Options:
- Mock the prune failure by testing with a path where git worktree prune would fail
- Test the struct shape directly to ensure warnings are propagated
- Consider testing at the MCP level where WorktreeListResponse serialization can be verified
