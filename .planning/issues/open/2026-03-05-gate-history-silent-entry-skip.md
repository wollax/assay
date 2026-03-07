# gate_history silently drops unreadable entries

**Source:** PR #57 review (error handling)
**Severity:** Important
**Area:** assay-mcp

## Description

In `gate_history` list mode, when `history::load` fails for a run ID, the entry is silently dropped with only a `tracing::warn`. The `total_runs` count won't match `runs.len()` with no signal to the agent about data completeness.

## Location

`crates/assay-mcp/src/server.rs` — `gate_history` method (~line 717-719)

## Suggested Fix

Add a `skipped_runs` count or `errors` array (like `spec_list` already does) so agents can reason about data completeness.
