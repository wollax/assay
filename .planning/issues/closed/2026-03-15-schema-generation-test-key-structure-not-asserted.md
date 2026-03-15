# Test: `schema_generation_produces_valid_json` should assert key structure

**Area:** crates/assay-core/src/evaluator.rs
**Severity:** Low
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

`schema_generation_produces_valid_json` only checks that the output parses as valid JSON. It does not assert any key structure (e.g. presence of `properties`, `required`, or specific field names). A schema regression such as a field rename would produce valid JSON and pass the test without detection.

## Suggested Fix

Assert that the parsed schema contains the expected top-level keys and at least the required field names:

```rust
let schema: serde_json::Value = serde_json::from_str(&json).unwrap();
assert!(schema["properties"]["criteria"].is_object());
assert!(schema["required"].as_array().unwrap().contains(&json!("criteria")));
```

## Category

testing
