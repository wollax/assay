# worktree-list-params-unused-field

**Source:** Phase 41 PR review
**Severity:** Suggestion
**File:** `crates/assay-mcp/src/server.rs:223`

## Description

`WorktreeListParams.worktree_dir` has a detailed Rust doc comment explaining it's unused, but the schemars description only says "reserved for future use". Either drop the field or echo the rationale in the schema.

## Suggested Fix

Synchronize the documentation between the Rust struct comment and the schemars description to ensure clarity for API consumers.
