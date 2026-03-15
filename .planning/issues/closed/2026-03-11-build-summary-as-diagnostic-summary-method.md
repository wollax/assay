# `build_summary` should be `DiagnosticSummary::from_diagnostics()`

**Area:** assay-types
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

`build_summary` is a free function that operates entirely on `DiagnosticSummary` data and a slice of `Diagnostic` values. Moving it to an associated function `DiagnosticSummary::from_diagnostics()` would improve cohesion, discoverability, and make the construction intent self-documenting at call sites.

## Suggested Fix

Replace the free `build_summary` function with an associated function on `DiagnosticSummary`:

```rust
impl DiagnosticSummary {
    pub fn from_diagnostics(diagnostics: &[Diagnostic]) -> Self { ... }
}
```

Update call sites accordingly.
