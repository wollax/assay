---
created: 2026-03-04T10:00
title: Enforcement validation block duplicated in validate() and validate_gates_spec()
area: assay-core
severity: important
files:
  - crates/assay-core/src/spec/mod.rs:131-152
  - crates/assay-core/src/spec/mod.rs:377-398
---

## Problem

The enforcement validation block checking `has_executable` and `has_required_executable` is identical in both the `validate()` and `validate_gates_spec()` methods, violating DRY principle.

## Solution

Extract the shared enforcement validation logic into a private helper method and call it from both sites.
