# Session timeout task captures unused wd_string

**Source:** PR #57 review (code quality)
**Severity:** Important
**Area:** assay-mcp

## Description

The `wd_string` variable is captured into the session timeout async closure but explicitly suppressed with `let _ = wd_string`. Dead code captured into a long-lived async task is a code smell.

## Location

`crates/assay-mcp/src/server.rs` — session timeout task (~line 510, 539)

## Suggested Fix

Either use it (set `working_dir` on the record) or remove the capture entirely.
