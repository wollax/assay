# session-create-worktree-path-absolute-validation

**Source:** Phase 41 PR review
**Severity:** Suggestion
**File:** `crates/assay-mcp/src/server.rs:1548`

## Description

`worktree_path` accepts any path without validating it's absolute. The schemars description says "Absolute path" but it's not enforced. Add `Path::is_absolute()` guard.

## Suggested Fix

Add validation to reject relative paths in `session_create`, or remove the "Absolute path" requirement from the schema if relative paths are acceptable.
