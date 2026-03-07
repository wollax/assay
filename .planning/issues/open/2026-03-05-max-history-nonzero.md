# max_history: Option<usize> allows Some(0) — consider NonZeroUsize

**Area:** types
**Severity:** suggestion
**Source:** Phase 15 PR review

## Description

`GatesConfig.max_history: Option<usize>` permits `Some(0)`, which is treated as "unlimited" by convention. Using `Option<NonZeroUsize>` would encode the invariant at the type level and prevent misconfiguration. `NonZeroUsize` serializes/deserializes transparently with serde.

**File:** `crates/assay-types/src/lib.rs`
