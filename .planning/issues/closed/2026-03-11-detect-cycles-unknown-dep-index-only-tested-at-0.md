# `detect_cycles` unknown-dep location indexing only tested for index 0

**Area:** assay-core/spec/validate.rs
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

The existing tests for unknown dependency detection in `detect_cycles` only verify the case where the unknown dependency is at `depends[0]`. The location/span calculation for unknown deps at other indices (e.g. `depends[1]`) is not covered, which could mask off-by-one errors in the indexing logic.

## Suggested Fix

Add a test where a spec has at least two `depends` entries and the *second* entry (`depends[1]`) is unknown. Assert that the resulting diagnostic correctly points to index 1.
