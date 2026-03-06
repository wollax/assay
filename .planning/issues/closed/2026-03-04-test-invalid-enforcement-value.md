---
created: 2026-03-04T10:00
title: No negative test for invalid enforcement value
area: assay-types
severity: suggestion
files:
  - crates/assay-types/src/enforcement.rs
---

## Problem

The test suite lacks a negative test case verifying that invalid enforcement values (e.g., `"strict"`, `"optional"`) are properly rejected during deserialization, leaving a gap in serde validation coverage.

## Solution

Add a test case that attempts to deserialize an invalid enforcement value and verifies the error handling.


## Resolution

Resolved in Phase 19 Plan 02 (2026-03-06). Tests added in the appropriate crate test modules.
