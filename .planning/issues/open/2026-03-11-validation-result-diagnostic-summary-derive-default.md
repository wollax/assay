# `ValidationResult` and `DiagnosticSummary` should derive `Default`

**Area:** assay-types/validation.rs
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

`ValidationResult` and `DiagnosticSummary` have natural empty/zero defaults but do not derive `Default`. This forces callers to construct them manually (e.g. `ValidationResult { diagnostics: vec![], .. }`) and prevents use with `Option::unwrap_or_default()` or `..Default::default()` spread syntax.

## Suggested Fix

Add `Default` to the derive list on both types:

```rust
#[derive(Debug, Clone, Default)]
pub struct ValidationResult { ... }

#[derive(Debug, Clone, Default)]
pub struct DiagnosticSummary { ... }
```
