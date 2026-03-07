---
created: 2026-03-05T00:00
title: regex_lite_match helper not independently tested
area: assay-core
provenance: phase-14-review
files:
  - crates/assay-core/src/history/mod.rs
---

## Problem

The `regex_lite_match()` helper function is used internally but has no dedicated unit tests. Its correctness is only verified indirectly through integration tests. If the regex logic breaks, tests may not catch it, or failures may be hard to diagnose.

## Solution

Add unit tests for `regex_lite_match()` covering:
1. Filenames that match the pattern
2. Filenames that don't match
3. Edge cases (empty string, special characters, etc.)

