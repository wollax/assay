# tool-count-in-docs-fragile

**Source:** Phase 41 PR review
**Severity:** Suggestion
**File:** `crates/assay-mcp/src/lib.rs:2` and `crates/assay-mcp/src/server.rs:3`

## Description

Hard-coded "Seventeen tools" in module docs is fragile and will go stale when tools are added.

## Suggested Fix

Either remove the count from the docs, use a dynamic mechanism to count tools, or document the last update date clearly so staleness is obvious.
