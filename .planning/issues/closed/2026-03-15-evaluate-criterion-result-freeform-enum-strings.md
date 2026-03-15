# `EvaluateCriterionResult.outcome` and `enforcement` re-serialize enums as freeform strings

**Area:** crates/assay-mcp/src/server.rs:660
**Severity:** Low
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

The private `EvaluateCriterionResult` response struct re-serializes `outcome` and `enforcement` as freeform `String` fields, even though `CriterionOutcome` and `Enforcement` already derive `Serialize`. This duplicates the serialization logic and means a rename of an enum variant would not be caught at compile time.

## Suggested Fix

Use the typed enum fields directly in the response struct:

```rust
struct EvaluateCriterionResult {
    outcome: CriterionOutcome,
    enforcement: Enforcement,
    ...
}
```

## Category

types
