# DiagnosticSummary::info naming inconsistency

**Source:** Phase 37 PR review (type-reviewer)
**Area:** assay-types/validation
**File(s):** crates/assay-types/src/validation.rs

## Description

The `DiagnosticSummary` struct uses `errors` and `warnings` (plural count nouns) for two of its fields, but the third field is named `info` (an uncountable mass noun). This inconsistency makes the API feel unpolished and creates a mismatch when destructuring or pattern-matching across the three fields. It is also a minor readability footgun since `summary.info` reads as a singular accessor rather than a count.

## Suggested Fix

Rename the field to `infos` to match the plural convention of `errors` and `warnings`, or if `infos` feels unnatural, adopt a uniform suffix such as `error_count`/`warning_count`/`info_count`.
