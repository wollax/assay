# `evaluator_schema_json()` could use `LazyLock` caching

**Area:** crates/assay-core/src/evaluator.rs:59
**Severity:** Low
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

`evaluator_schema_json()` calls `schema_for!` on every invocation. The schema is constant per binary (it depends only on the type, not runtime data), so repeated calls do redundant work. In the MCP server hot path this is called for every evaluate request.

## Suggested Fix

Cache the result in a `LazyLock<String>`:

```rust
static EVALUATOR_SCHEMA: LazyLock<String> = LazyLock::new(|| {
    serde_json::to_string(&schema_for!(EvaluatorInput)).expect("schema serialization")
});

pub fn evaluator_schema_json() -> &'static str {
    &EVALUATOR_SCHEMA
}
```

## Category

performance
