# `RecoverySummary` Has No Way to Signal the 100-Session Cap Was Hit

## Description

`recover_stale_sessions` stops processing after 100 sessions but `RecoverySummary` has no field that reflects this. Callers (e.g., the MCP server startup log) cannot warn the operator that the scan was truncated and some stale sessions may have been skipped. Adding a `truncated: bool` field (set to `true` when the cap is reached) would make the truncation observable without changing the existing numeric fields.

## File Reference

`crates/assay-core/src/work_session.rs` — `RecoverySummary`, `recover_stale_sessions`

## Category

observability / correctness
