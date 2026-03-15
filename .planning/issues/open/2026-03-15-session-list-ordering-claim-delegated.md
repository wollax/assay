# session-list-ordering-claim-delegated

**Source:** Phase 41 PR review
**Severity:** Suggestion
**File:** `crates/assay-mcp/src/server.rs:1654`

## Description

Tool description claims "oldest first, ULID order" but the ordering is entirely delegated to `list_sessions`. Should qualify the claim.

## Suggested Fix

Either update the tool description to note that ordering is delegated to `list_sessions`, or document the guaranteed sort order at the point of delegation.
