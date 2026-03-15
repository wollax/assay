# session-create-spec-validation-overhead

**Source:** Phase 41 PR review
**Severity:** Suggestion
**File:** `crates/assay-mcp/src/server.rs:1541`

## Description

`load_spec_entry_mcp` parses the full spec just to validate existence in `session_create`. A lighter existence check would avoid coupling to spec parse errors.

## Suggested Fix

Decouple the existence check from full spec parsing. Consider adding a lightweight spec existence check that doesn't parse the full TOML, or refactor to separate the validation concern.
