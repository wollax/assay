# `EvaluatorError` missing `Io` variant

**Area:** crates/assay-core/src/error.rs
**Severity:** Medium
**Source:** PR #43 review (phase-43-gate-evaluate-schema)

## Description

stdin write failures and working directory errors are currently repackaged as `EvaluatorError::Crash`. This conflates local I/O failures with subprocess crashes, making it harder for callers to distinguish the two cases and to provide accurate diagnostics.

## Suggested Fix

Add a dedicated `Io(std::io::Error)` variant to `EvaluatorError`:

```rust
#[error("evaluator I/O error: {0}")]
Io(#[from] std::io::Error),
```

Use `Io` for stdin write errors and working directory resolution failures; reserve `Crash` for actual subprocess exit failures.

## Category

error-handling
