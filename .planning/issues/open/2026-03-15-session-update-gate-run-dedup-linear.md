# session-update-gate-run-dedup-linear

**Source:** Phase 41 PR review
**Severity:** Suggestion
**File:** `crates/assay-mcp/src/server.rs:1629`

## Description

Gate run ID deduplication uses `Vec::contains` (O(n²)). Fine for small lists but should document the assumption or use a HashSet.

## Suggested Fix

Either add a documentation comment explaining the expected size constraints, or refactor to use a `HashSet` for O(n) deduplication.
