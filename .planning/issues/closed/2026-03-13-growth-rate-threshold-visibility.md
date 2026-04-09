---
created: 2026-03-13T10:45
title: Make MIN_TURNS_FOR_GROWTH_RATE visible to type consumers
area: types
provenance: local
files:
  - crates/assay-core/src/context/tokens.rs:19
  - crates/assay-types/src/context.rs:354
---

## Problem

`MIN_TURNS_FOR_GROWTH_RATE` (5) is a private constant in `assay-core` but controls a concept that belongs to `GrowthRate` in `assay-types`. Consumers of the type who want to explain the "absent when fewer than 5 turns" threshold cannot access it programmatically.

## Solution

Consider making it a `pub const` on `GrowthRate` in `assay-types`, or at least doc-referencing the value from the type's documentation.
