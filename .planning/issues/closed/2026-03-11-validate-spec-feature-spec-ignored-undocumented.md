# `validate_spec` ignores `FeatureSpec` entirely without explanation

**Area:** assay-core/spec/validate.rs
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

`validate_spec` has no handling for the `FeatureSpec` variant — it is silently skipped. Without a comment, it is impossible to tell whether this is an intentional design decision (e.g. `FeatureSpec` requires no validation) or an unfinished implementation.

## Suggested Fix

Add an inline comment at the `FeatureSpec` match arm (or wherever the omission occurs) explicitly stating that the variant is intentionally not validated and briefly explaining why, for example:

```rust
// FeatureSpec has no validatable fields at this time; skip.
SpecKind::Feature(_) => {}
```
