# SaveResult should derive Clone, PartialEq, Eq

**Area:** core/history
**Severity:** suggestion
**Source:** Phase 15 PR review

## Description

`SaveResult` only derives `Debug`. Both fields (`PathBuf` and `usize`) support `Clone + Eq`. Adding `#[derive(Debug, Clone, PartialEq, Eq)]` costs nothing and makes the type more useful in tests and callers.

**File:** `crates/assay-core/src/history/mod.rs`
