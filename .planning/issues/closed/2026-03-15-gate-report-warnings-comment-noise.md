# gate-report-warnings-comment-noise

**Source:** Phase 41 PR review
**Severity:** Suggestion
**File:** `crates/assay-mcp/src/server.rs:437`

## Description

`GateReportResponse.warnings` comment is an implementation note that will go stale. Replace with standard form.

## Suggested Fix

Replace the implementation-level comment with a user-facing description of what warnings represent and when they occur.
