# gate_run_with_timeout test doesn't test actual timeout behavior

**Source:** PR review (Phase 19)
**Area:** crates/assay-mcp/tests/mcp_handlers.rs
**Priority:** low

The `gate_run_with_timeout` test uses `cmd = "echo ok"` (instant) with `timeout: Some(10)`. It only tests that the timeout parameter is accepted, not that a slow command actually times out. Consider adding a test with `sleep 60` and `timeout: 1` to verify timeout triggers.
