---
created: 2026-03-04T10:00
title: Extract shared logic from evaluate_all and evaluate_all_gates
area: assay-core
severity: important
files:
  - crates/assay-core/src/gate/mod.rs:84-226
---

## Problem

`evaluate_all()` and `evaluate_all_gates()` contain duplicated criterion evaluation logic (lines 84-226). This violates DRY and makes future fixes require changes in two places.

## Solution

Extract shared criterion iteration and evaluation logic into a reusable `evaluate_criteria_iter()` function that both callers can use.
