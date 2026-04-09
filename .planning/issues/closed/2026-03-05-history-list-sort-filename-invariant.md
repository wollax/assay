---
created: 2026-03-05T00:00
title: list() sort relies on filename format — document the invariant inline
area: assay-core
provenance: phase-14-review
files:
  - crates/assay-core/src/history/mod.rs
---

## Problem

The `list()` function sorts records by filename, which only produces correct chronological order if filenames follow a specific format (timestamps in sortable order). This dependency is undocumented, making it fragile if filenames are ever changed or if entries don't follow the convention.

## Solution

Document the filename format invariant in an inline comment in the `list()` function. Consider also documenting the invariant in the `save()` function (which generates filenames) to keep the two in sync.

