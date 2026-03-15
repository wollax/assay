# persisted field derives from warnings.is_empty() — fragile

**Area:** crates/assay-mcp/src/server.rs
**Severity:** suggestion
**Source:** PR review (phase 35)

## Description

`persisted: warnings.is_empty()` in the gate_finalize handler means any future non-persistence warning would spuriously set `persisted: false`. Use an explicit `let persisted = save_result.is_ok()` local instead.

## Location

`crates/assay-mcp/src/server.rs` — GateFinalizeResponse construction in gate_finalize handler.
