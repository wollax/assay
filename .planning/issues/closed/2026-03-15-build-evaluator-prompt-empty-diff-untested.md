# Test: `build_evaluator_prompt` with empty string diff should match `None` behavior

**Area:** crates/assay-core/src/evaluator.rs
**Severity:** Low
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

There is no test verifying that `build_evaluator_prompt` with `diff: Some("")` produces the same prompt as `diff: None`. If the empty-string case is intended to be equivalent to no diff, this equivalence should be asserted explicitly. If they are intentionally different, the behavior should be documented and tested.

## Suggested Fix

Add a test:

```rust
let with_none = build_evaluator_prompt(..., None, ...);
let with_empty = build_evaluator_prompt(..., Some(""), ...);
assert_eq!(with_none, with_empty);
```

Or, if the behaviors differ, add separate tests documenting the distinction.

## Category

testing
