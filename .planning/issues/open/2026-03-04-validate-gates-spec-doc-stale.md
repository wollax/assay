---
created: 2026-03-04T10:00
title: validate_gates_spec doc comment doesn't mention new enforcement validation rule
area: assay-core
severity: important
files:
  - crates/assay-core/src/spec/mod.rs:332-335
---

## Problem

The doc comment for `validate_gates_spec()` doesn't mention the enforcement validation rule that requires certain enforcement levels to have executable criteria, leaving the documentation incomplete and out of sync with the implementation.

## Solution

Update the doc comment to document the enforcement validation rule alongside the existing validation rules.
