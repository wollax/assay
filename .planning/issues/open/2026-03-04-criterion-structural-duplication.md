---
created: 2026-03-04T10:00
title: Reduce structural duplication between GateCriterion and Criterion
area: assay-types
severity: important
files:
  - crates/assay-types/src/gates_spec.rs
  - crates/assay-types/src/criterion.rs
---

## Problem

`GateCriterion` and `Criterion` share nearly identical structure, leading to duplication and maintenance burden. Changes to shared fields must be applied in two places.

## Solution

Consider composition patterns (newtype, shared trait, or extracting common fields into a shared struct) to reduce duplication while maintaining type clarity.
