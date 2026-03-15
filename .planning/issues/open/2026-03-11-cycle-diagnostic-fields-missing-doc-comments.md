# `CycleDiagnostic` fields `diagnostic` and `specs` lack doc comments

**Area:** assay-core/spec/validate.rs
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

`CycleDiagnostic` has two fields, `diagnostic` and `specs`, with no doc comments. The `specs` field has asymmetric semantics that are non-obvious: it represents the ordered path of spec slugs forming the cycle, not the full set of affected specs. This distinction is worth documenting explicitly.

## Suggested Fix

Add doc comments to both fields:

```rust
/// The diagnostic emitted for this cycle, including location and message.
diagnostic: Diagnostic,
/// The ordered sequence of spec slugs forming the cycle path (first == last for a closed cycle).
specs: Vec<String>,
```
