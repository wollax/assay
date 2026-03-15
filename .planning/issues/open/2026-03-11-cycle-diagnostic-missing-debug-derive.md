# `CycleDiagnostic` lacks `#[derive(Debug)]`

**Area:** assay-core/spec/validate.rs
**Severity:** suggestion
**Source:** PR review (Phase 37)

## Description

`CycleDiagnostic` does not derive `Debug`. This makes it harder to inspect in test failures, log output, and `dbg!()` calls. All internal structs should derive `Debug` as a baseline.

## Suggested Fix

Add `Debug` to the derive list on `CycleDiagnostic`:

```rust
#[derive(Debug)]
struct CycleDiagnostic { ... }
```
