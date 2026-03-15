# `Diagnostic` should derive `Hash` for dedup capability

**Area:** assay-types/validation.rs
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

`Diagnostic` currently derives `PartialEq + Eq` but not `Hash`. Without `Hash`, it cannot be used as a key in a `HashMap` or inserted into a `HashSet`, which prevents straightforward deduplication of diagnostics by value.

## Suggested Fix

Add `Hash` to the derive list on `Diagnostic`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Diagnostic { ... }
```
