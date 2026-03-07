# spec_get silently swallows feature spec load errors

**Source:** PR #57 review (error handling)
**Severity:** Important
**Area:** assay-mcp

## Description

In `spec_get`, `load_feature_spec` errors are swallowed with `.ok()`. A malformed `spec.md` returns `feature_spec: null` with no indication anything went wrong. The agent has no way to know whether the feature spec is absent or failed to parse.

## Location

`crates/assay-mcp/src/server.rs` — `spec_get` method (~line 403)

## Suggested Fix

Return a `feature_spec_error` field, or at minimum log a warning when the load fails.
