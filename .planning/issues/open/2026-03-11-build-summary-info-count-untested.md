# `build_summary` Info count never exercised in tests

**Area:** assay-core/spec/validate.rs
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

Test coverage for `build_summary` only exercises `Error` and `Warning` severity counts. The `Info` severity path is never exercised, meaning the info-count accumulation logic could silently regress without any test catching it.

## Suggested Fix

Add a test case that includes at least one `Diagnostic` with `Severity::Info` and asserts that the `info` field of the resulting `DiagnosticSummary` reflects the correct count.
