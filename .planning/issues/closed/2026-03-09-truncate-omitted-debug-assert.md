---
created: 2026-03-09T21:00
title: Add debug_assert for omitted subtraction invariant
area: core
provenance: github:wollax/assay#77
files:
  - crates/assay-core/src/gate/mod.rs
---

## Problem

The `omitted = original_bytes - head.len() - tail.len()` subtraction in `truncate_head_tail` is safe but the invariant that `head.len() + tail.len() <= original_bytes` is non-obvious to readers.

## Solution

Add `debug_assert!(head.len() + tail.len() <= original_bytes)` before the subtraction to make the invariant explicit and catch violations in debug builds.
