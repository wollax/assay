---
created: 2026-03-04T10:00
title: Schema roundtrip test should clarify scope vs spec validate()
area: assay-types
severity: suggestion
files:
  - crates/assay-types/tests/schema_roundtrip.rs
---

## Problem

The schema roundtrip test doesn't clarify its scope relative to the stricter validation rules in `spec.validate()`. Schema roundtrip coverage is limited to serde compatibility and doesn't guarantee that deserialized specs pass full domain validation.

## Solution

Add a doc comment explaining that schema roundtrip tests verify serde compatibility only, and that full validation must be performed by `spec.validate()` or `spec.validate_gates_spec()`.
