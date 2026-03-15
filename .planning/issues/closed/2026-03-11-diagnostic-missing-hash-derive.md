> **Closed:** 2026-03-15 — Duplicate of `2026-03-11-diagnostic-derive-hash.md`. Resolved in Phase 45 Plan 04.


# Diagnostic missing Hash derive

**Source:** Phase 37 PR review (type-reviewer)
**Area:** assay-types/validation
**File(s):** crates/assay-types/src/validation.rs

## Description

`Diagnostic` derives `PartialEq` and `Eq` but not `Hash`. This combination prevents the type from being stored in a `HashSet` or used as a `HashMap` key, which are natural choices for deduplicating diagnostics. Any caller that needs deduplication must instead use a `BTreeSet` (requiring `Ord`) or deduplicate manually with a `Vec`, both of which are more awkward.

## Suggested Fix

Add `#[derive(Hash)]` to `Diagnostic` (and ensure all its fields also implement `Hash`).