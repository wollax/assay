# session-list-entry-missing-worktree-path

**Source:** Phase 41 PR review
**Severity:** Suggestion
**File:** `crates/assay-mcp/src/server.rs:598`

## Description

`SessionListEntry` omits `worktree_path`. Agents listing sessions to find which worktree to resume work in must issue separate `session_get` per entry.

## Suggested Fix

Add `worktree_path` to `SessionListEntry` to allow agents to batch-discover worktree locations without extra `session_get` calls.
