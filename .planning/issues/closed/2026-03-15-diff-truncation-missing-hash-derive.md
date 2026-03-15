# DiffTruncation missing Hash derive

**Source:** PR review (Phase 44)
**Severity:** Suggestion
**File:** crates/assay-types/src/gate_run.rs

`DiffTruncation` derives `PartialEq + Eq` but not `Hash`. Other result types with the same derive set include `Hash`. Since this is a new type, adding `Hash` now is cheaper than retrofitting later.
