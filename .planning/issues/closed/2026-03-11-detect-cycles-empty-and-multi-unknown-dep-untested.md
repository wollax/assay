# `detect_cycles` empty-input and multi-unknown-dep cases untested

**Area:** assay-core/spec/validate.rs
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

The test suite for `detect_cycles` does not cover the empty-input case (no specs) or the case where a single spec has multiple unknown dependencies. These boundary conditions may conceal off-by-one errors or incorrect diagnostic generation.

## Suggested Fix

Add two tests:
1. Pass an empty spec map to `detect_cycles` and assert no diagnostics are returned.
2. Pass a spec with two or more unknown `depends` entries and assert that each produces a distinct diagnostic with the correct location.
