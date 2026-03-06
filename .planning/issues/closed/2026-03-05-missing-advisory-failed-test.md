# Missing test for advisory-failed-but-not-blocked scenario

**Source:** PR #57 review (tests)
**Severity:** Important
**Area:** assay-mcp

## Description

No test verifies the key behavioral distinction: when `advisory_failed > 0` and `required_failed == 0`, `blocked` should be `false`. The existing `test_format_gate_response_enforcement_counts` only tests the all-pass case.

## Location

`crates/assay-mcp/src/server.rs` — test module

## Suggested Fix

Add a test with mixed enforcement where advisory criteria fail but required criteria pass, asserting `blocked: false`.


## Resolution

Resolved in Phase 19 Plan 02 (2026-03-06). Tests added in the appropriate crate test modules.
