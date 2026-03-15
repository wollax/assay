# Test: validate_spec_with_dependencies needs more coverage

**Source:** Phase 37 PR review (test-reviewer)
**Area:** assay-core/spec
**File(s):** crates/assay-core/src/spec/validate.rs

## Description

`validate_spec_with_dependencies`, the public entry point for validation, has zero test coverage. The important branches — no-deps early return, a valid dependency graph, a cyclic dependency graph, and a scan failure — are all untested. Because this is the entry point that callers actually use, bugs here could silently produce incorrect results.

## Suggested Fix

Add tests for each major branch: the no-dependencies short-circuit, a passing multi-spec graph, a cycle being detected and surfaced as a diagnostic, and a simulated scan failure returning an error.
