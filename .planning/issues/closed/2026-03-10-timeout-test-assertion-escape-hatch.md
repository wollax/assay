# Timeout Test Assertion Escape Hatch Weakens Coverage

## Description

`test_gate_run_params_invalid_timeout_type` uses `|| msg.contains("invalid value")` as an escape hatch in its assertion, which weakens the test. The assertion should be tightened to require the specific expected error message without the fallback branch.

## File Reference

`crates/assay-mcp/src/server.rs` (`test_gate_run_params_invalid_timeout_type`)

## Category

testing

## Severity

medium
