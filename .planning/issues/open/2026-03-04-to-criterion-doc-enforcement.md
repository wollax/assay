---
created: 2026-03-04T10:00
title: to_criterion doc should mention enforcement is preserved
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/gate/mod.rs
---

## Problem

The `to_criterion` method doc comment doesn't mention that the `enforcement` field is preserved during conversion, which could confuse callers about the transformation semantics.

## Solution

Add clarifying doc text explaining that `enforcement` is preserved when converting to `Criterion`.
