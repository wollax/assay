---
created: 2026-03-04T00:00
title: Add tally() method to StreamCounters
area: assay-cli
provenance: github:wollax/assay#PR-review
files:
  - crates/assay-cli/src/main.rs
---

## Problem

Counter increments are scattered throughout `stream_criterion`, with repeated patterns like `counters.passed += 1` or `counters.failed += 1` depending on the result status. This reduces readability and increases likelihood of inconsistent tallying logic.

## Solution

Add a `tally(result)` method to `StreamCounters` that accepts a result enum and increments the appropriate counter (passed/failed/skipped). This centralizes the counting logic and makes the intent clearer at call sites.
