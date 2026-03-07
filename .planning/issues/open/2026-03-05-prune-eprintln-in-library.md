# prune() uses eprintln! directly in library code

**Area:** core/history
**Severity:** suggestion
**Source:** Phase 15 PR review

## Description

`prune()` in `assay-core/src/history/mod.rs` calls `eprintln!` directly for per-file deletion warnings. Library code should not write to stderr — it should return structured information about partial failures or use `tracing` for diagnostics.

**File:** `crates/assay-core/src/history/mod.rs`
