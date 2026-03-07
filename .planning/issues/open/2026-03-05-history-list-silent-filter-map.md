---
created: 2026-03-05T00:00
title: history list() silently drops unreadable directory entries
area: assay-core
provenance: phase-14-review
files:
  - crates/assay-core/src/history/mod.rs
---

## Problem

The `list()` function uses `.filter_map(|entry| entry.ok())` which silently drops any directory entries that fail to read (permission denied, symlink loop, etc.). Callers have no visibility into data loss and may assume they have a complete list when entries are actually missing.

## Solution

Either:
1. Return an error if any directory entry fails to read (fail-fast), or
2. Log/report skipped entries and allow partial results with warnings

Choose based on whether incomplete history is acceptable in the use case.

