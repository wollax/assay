# `build_summary` uses imperative loop instead of functional combinators

**Area:** assay-core/spec/validate.rs
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

`build_summary` accumulates counts using a mutable loop, which goes against the project convention of preferring functional and declarative patterns. The same result can be expressed more clearly using `.fold()` or separate `.filter().count()` passes.

## Suggested Fix

Rewrite `build_summary` using functional iterator combinators:

```rust
let errors   = diagnostics.iter().filter(|d| d.severity == Severity::Error).count();
let warnings = diagnostics.iter().filter(|d| d.severity == Severity::Warning).count();
let infos    = diagnostics.iter().filter(|d| d.severity == Severity::Info).count();
```

Or with a single `.fold()` if performance is a concern.
