# `recover_skips_non_agent_running` Test Missing `skipped == 0` and `errors == 0` Assertions

## Description

The `recover_skips_non_agent_running` test verifies that non-`AgentRunning` sessions are not recovered, but it does not assert that `summary.skipped == 0` or `summary.errors == 0`. Without these assertions the test would pass even if the function accidentally incremented those counters, masking a bug in the skipping logic. Adding the missing assertions tightens the contract.

## File Reference

`crates/assay-core/src/work_session.rs` — `recover_skips_non_agent_running` test

## Category

testing / assertions
