---
created: 2026-03-04T10:00
title: Add full roundtrip test for FeatureSpec
area: assay-types
severity: important
files:
  - crates/assay-types/tests/schema_roundtrip.rs
---

## Problem

`FeatureSpec` has only one minimal roundtrip test, leaving most fields untested for serialization/deserialization correctness. This risks breaking changes going unnoticed.

## Solution

Add comprehensive roundtrip test covering all FeatureSpec fields and variants, similar to existing roundtrip tests for other spec types.
