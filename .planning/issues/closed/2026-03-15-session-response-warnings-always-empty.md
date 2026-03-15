# session-response-warnings-always-empty

**Source:** Phase 41 PR review
**Severity:** Suggestion
**File:** `crates/assay-mcp/src/server.rs`

## Description

`SessionCreateResponse`, `SessionGetResponse`, `SessionUpdateResponse` always construct `warnings: Vec::new()`. Add TODO comments noting what warnings are anticipated for future use.

## Suggested Fix

Add documentation comments to the warning fields in these response types explaining what future warning scenarios they might capture.
