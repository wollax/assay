# Phase 24: MCP History Persistence Fix

## Problem

The v0.2.0 milestone audit identified an integration asymmetry:

- **CLI** `handle_gate_run` always calls `save_run_record()` after evaluation, persisting results to history
- **MCP** `gate_run` handler only persists history when the spec contains agent criteria (via `gate_finalize` flow)
- For **command-only specs** evaluated via MCP, no history record is created

This means an agent calling `gate_run` on a command-only spec, then calling `gate_history`, will not see the run.

## Location

`crates/assay-mcp/src/server.rs` lines 442-568 (gate_run handler)

## Reference

- CLI history save: `crates/assay-cli/src/main.rs` (`save_run_record` call in `handle_gate_run`)
- History module: `crates/assay-core/src/history.rs`
- Audit finding: `.planning/v0.2.0-MILESTONE-AUDIT.md` (Integration Gap section)

## Scope

1. Add `history::save()` call to MCP `gate_run` handler for command-only specs (no agent criteria)
2. Add integration test verifying history persistence for command-only MCP gate runs
3. Verify `gate_history` returns the persisted run
