---
created: 2026-03-04T10:00
title: No TOML roundtrip test for enforcement field on Criterion
area: assay-types
severity: suggestion
files:
  - crates/assay-types/src/criterion.rs
---

## Problem

The test suite lacks a TOML roundtrip test for the `enforcement` field on `Criterion`, leaving a gap in coverage for serialization and deserialization of the enforcement attribute in TOML format.

## Solution

Add a TOML roundtrip test case that serializes and deserializes a `Criterion` with `enforcement` values to verify correct TOML handling.


## Resolution

Resolved in Phase 19 Plan 02 (2026-03-06). Tests added in the appropriate crate test modules.
