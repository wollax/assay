# session-create-agent-model-example-stale

**Source:** Phase 41 PR review
**Severity:** Suggestion
**File:** `crates/assay-mcp/src/server.rs:288`

## Description

`agent_model` example `"claude-sonnet-4-20250514"` is version-locked and will go stale.

## Suggested Fix

Use a more generic example like `"claude-sonnet-4"` or note that version specifiers are optional/should match currently available models.
