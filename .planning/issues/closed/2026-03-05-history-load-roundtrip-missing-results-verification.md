---
created: 2026-03-05T00:00
title: test_load_roundtrip doesn't verify summary.results content
area: assay-core
provenance: phase-14-review
files:
  - crates/assay-core/src/history/mod.rs
---

## Problem

`test_load_roundtrip` creates test data with an empty `results` vector, saves it, loads it back, and asserts equality. However, the test doesn't verify that the results are actually being serialized and deserialized correctly — only that they're empty. This is a weak test that wouldn't catch regressions in results handling.

## Solution

Populate the test data with non-empty results (at least one criterion with pass/fail states) and verify they round-trip correctly.



## Resolution

Resolved in Phase 19 Plan 02 (2026-03-06). Tests added in the appropriate crate test modules.
