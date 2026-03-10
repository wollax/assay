---
created: 2026-03-09T21:15
title: Missing test for parse_worktree_list with malformed input
area: core
provenance: github:wollax/assay#97
files:
  - crates/assay-core/src/worktree.rs:68-92
---

## Problem

`parse_worktree_list` with malformed input (missing HEAD line, worktree line without path) is not tested. The `filter_map` with `?` operators silently drops malformed entries, which is correct but unverified.

## Solution

Add a unit test with malformed porcelain input (e.g., missing HEAD) and verify entries are silently skipped.
