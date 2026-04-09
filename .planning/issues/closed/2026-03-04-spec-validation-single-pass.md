---
created: 2026-03-04T10:00
title: Two-pass has_executable / has_required_executable validation could be single pass
area: assay-core
severity: suggestion
files:
  - crates/assay-core/src/spec/mod.rs
---

## Problem

The enforcement validation logic performs two separate passes through criteria to check `has_executable` and `has_required_executable`, which is inefficient when a single pass could accomplish both checks simultaneously.

## Solution

Refactor the validation to use a single pass through criteria, tracking both executable and required_executable states in one iteration for better performance.
