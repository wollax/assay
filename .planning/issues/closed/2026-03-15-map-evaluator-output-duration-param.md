# `map_evaluator_output` duration_ms parameter should accept `Duration`

**Area:** crates/assay-core/src/evaluator.rs:219
**Severity:** Low
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

`map_evaluator_output` takes `duration_ms: u64` but every caller converts from a `std::time::Duration` at the call site, pushing the lossy `as_millis() as u64` cast outward. The function signature should accept a `Duration` directly and perform the conversion internally, keeping the lossy cast in one place.

## Suggested Fix

Change the parameter type:

```rust
fn map_evaluator_output(
    output: EvaluatorOutput,
    duration: Duration,
    ...
) -> EvaluatorSummary {
    let duration_ms = duration.as_millis() as u64;
    ...
}
```

## Category

api-ergonomics
