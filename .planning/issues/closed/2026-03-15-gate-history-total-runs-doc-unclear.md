# gate-history-total-runs-doc-unclear

**Source:** Phase 41 PR review
**Severity:** Suggestion
**File:** `crates/assay-mcp/src/server.rs:480`

## Description

`total_runs` doc says "before limit AND outcome filter are applied" but should clarify it counts raw file IDs before deserialization.

## Suggested Fix

Expand the documentation to explain the exact counting semantics: raw file enumeration vs deserialized runs.
