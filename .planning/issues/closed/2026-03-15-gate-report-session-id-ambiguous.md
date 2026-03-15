# gate-report-session-id-ambiguous

**Source:** Phase 41 PR review
**Severity:** Suggestion
**File:** `crates/assay-mcp/src/server.rs:113`

## Description

`GateReportParams.session_id` description says "Session ID returned by gate_run" without clarifying it's an in-memory AgentSession ID, not a WorkSession ID.

## Suggested Fix

Clarify in the schema description that this is the in-memory AgentSession ID used during gate evaluation, not the persisted WorkSession ID.
