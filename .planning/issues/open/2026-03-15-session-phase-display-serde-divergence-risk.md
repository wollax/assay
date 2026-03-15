# session-phase-display-serde-divergence-risk

**Source:** Phase 41 PR review
**Severity:** Suggestion
**File:** `crates/assay-mcp/src/server.rs`

## Description

SessionPhase `Display` impl and serde `rename_all` both produce snake_case. If a variant is added and Display is updated differently from serde, response phase strings would silently diverge. Now mitigated by using SessionPhase directly in response structs (I3 fix), but Display is still used elsewhere.

## Suggested Fix

Document the relationship between the Display impl and serde serialization, or consolidate them to prevent future drift. Consider adding a test that verifies Display and serde output match.
