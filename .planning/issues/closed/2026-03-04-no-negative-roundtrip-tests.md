---
created: 2026-03-04T10:00
title: Add negative-case roundtrip tests for invalid specs
area: assay-types
severity: suggestion
files:
  - crates/assay-types/tests/schema_roundtrip.rs
---

## Problem

Roundtrip tests only verify successful serialization/deserialization. Negative cases (invalid specs that should fail to deserialize) are not tested, leaving error handling untested.

## Solution

Add test cases for invalid specs that should fail deserialization, verifying error messages are meaningful and consistent.


## Resolution

Resolved in Phase 19 Plan 02 (2026-03-06). Tests added in the appropriate crate test modules.
