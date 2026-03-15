# GateHistoryEntry missing required_passed/advisory_passed fields

**Source:** PR #57 review (type design)
**Severity:** Important
**Area:** assay-mcp

## Description

`GateHistoryEntry` has `required_failed` and `advisory_failed` but omits `required_passed` and `advisory_passed`. This is asymmetric with `GateRunResponse` which has all four counts. An agent comparing a current run against history cannot determine how many required checks existed.

## Location

`crates/assay-mcp/src/server.rs` — `GateHistoryEntry` struct

## Suggested Fix

Add `required_passed: usize` and `advisory_passed: usize` fields to `GateHistoryEntry` and populate them from `record.summary.enforcement`.
