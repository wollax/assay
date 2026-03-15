# get-info-session-vs-gate-workflow-clarity

**Source:** Phase 41 PR review
**Severity:** Suggestion
**File:** `crates/assay-mcp/src/server.rs:1757`

## Description

`get_info` instructions don't fully explain when to use `session_create` vs `gate_run` (both create sessions of different types).

## Suggested Fix

Clarify the use cases: explain that `session_create` initiates an agent session for long-running work, while `gate_run` runs ephemeral in-memory evaluation. Document which workflow to choose based on the agent's task.
