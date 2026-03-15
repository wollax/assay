# `CriterionOutcome` missing `Hash` derive

**Area:** crates/assay-types/src/evaluator.rs:13
**Severity:** Low
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

`CriterionOutcome` derives `Copy + Eq` but not `Hash`, which is inconsistent with `Enforcement` and `GateKind` in the same codebase. Without `Hash`, the type cannot be used as a map key or in a `HashSet` without a wrapper.

## Suggested Fix

Add `Hash` to the derive list alongside the existing `Eq`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum CriterionOutcome { ... }
```

## Category

types
