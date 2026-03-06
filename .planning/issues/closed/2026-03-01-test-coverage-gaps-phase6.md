---
title: "Test coverage gaps from Phase 6 PR review"
area: assay-core
priority: medium
source: PR review #27
---

# Test Coverage Gaps from Phase 6 PR Review

## Problem

Phase 6 spec module tests miss several edge cases identified during PR review:

1. **scan duplicate detection first-wins semantics not asserted** — test verifies duplicate produces an error, but doesn't assert the first file's spec is kept and the second is removed
2. **scan empty directory not tested** — `scan()` on a valid but empty directory should return `ScanResult { specs: [], errors: [] }`
3. **validate whitespace-only criterion name not tested** — `validate()` catches whitespace-only spec names but no test for whitespace-only criterion names
4. **validate multi-error within criteria list not tested** — multiple invalid criteria (e.g., 2 empty names) should collect all errors, not just the first

Additional gaps from second review:

5. **validate duplicate criterion test doesn't assert error count or field index** — only checks message contains "dup", doesn't verify `errors.len() == 1` or `errors[0].field == "criteria[1].name"`
6. **SpecError::Display has no test** — the `"{field}: {message}"` format is user-facing but unverified
7. **Schema roundtrip doesn't test empty description** — `skip_serializing_if = "String::is_empty"` path untested
8. **`format_criteria_type` ANSI overhead constant untested** — the hard-coded `+9` padding relies on escape sequence byte lengths verified only by inspection

## Solution

Add ~8 tests to `crates/assay-core/src/spec/mod.rs` and `crates/assay-cli/src/main.rs` covering these edge cases.


## Resolution

Partially resolved in Phase 19 Plan 02 (2026-03-06). Key gaps addressed: GateKind unknown variant test, GateResult JSON roundtrip with skip fields, Criterion deser failure, scan empty directory, SpecError Display format. Remaining items (assert_eq Display brittleness, whitespace-only criterion name, multi-error criteria) are low-priority suggestions.
