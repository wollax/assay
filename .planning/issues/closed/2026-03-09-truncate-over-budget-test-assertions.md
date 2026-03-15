---
created: 2026-03-09T21:00
title: Strengthen truncate_head_tail_over_budget test assertions
area: core
provenance: github:wollax/assay#77
files:
  - crates/assay-core/src/gate/mod.rs
---

## Problem

The `truncate_head_tail_over_budget` test only checks `truncated == true` and `output.len() < input.len()`. These assertions are too weak to catch regressions in the truncation logic.

## Solution

Also assert that the truncation marker is present in the output and that the non-marker content length approximates the budget.
