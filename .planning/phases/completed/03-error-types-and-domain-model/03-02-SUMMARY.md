---
phase: 03-error-types-and-domain-model
plan: 02
subsystem: error-handling
tags: [thiserror, error-types, non-exhaustive, result-alias]

dependency-graph:
  requires: []
  provides:
    - AssayError enum with context-rich Io variant
    - Result<T> type alias for Assay operations
    - Public re-exports from assay-core root
  affects:
    - 05-config-and-initialization (will use AssayError::Io for file operations)
    - 06-spec-files (will add spec-related error variants)
    - 07-gate-evaluation (will add gate-related error variants)

tech-stack:
  added: []
  patterns:
    - "Context-rich errors: structured fields (operation, path, source) instead of formatted strings"
    - "No #[from] on Io — callers must provide context via map_err"
    - "#[non_exhaustive] on error enum for forward-compatible variant additions"
    - "thiserror 2.x #[error] for Display, automatic source via field name"

key-files:
  created:
    - crates/assay-core/src/error.rs
  modified:
    - crates/assay-core/src/lib.rs

decisions:
  - id: error-io-structured-fields
    decision: "Io variant carries typed PathBuf + String fields, not pre-formatted strings"
    rationale: "Enables programmatic inspection of error context by consumers"
  - id: error-no-from-io
    decision: "No #[from] on Io variant"
    rationale: "Forces callers to add operation+path context at every call site"
  - id: error-non-exhaustive
    decision: "#[non_exhaustive] on AssayError"
    rationale: "Adding variants in future phases is non-breaking for downstream matchers"
  - id: error-add-as-consumed
    decision: "Only Io variant implemented now"
    rationale: "Variants added when downstream code actually needs them, not speculatively"

metrics:
  duration: ~3m
  completed: 2026-03-01
---

# Phase 03 Plan 02: AssayError and Result Summary

Context-rich AssayError with thiserror 2.x; Io variant carries operation (String), path (PathBuf), and source (io::Error) as structured fields, producing Display output like "reading config at `/tmp/config.toml`: No such file or directory". Result<T> alias and public re-exports from assay-core root.

## What Was Done

### Task 1: Implement AssayError and Result type alias

Created `crates/assay-core/src/error.rs` with:

- `AssayError` enum deriving `Debug` and `thiserror::Error`
- `#[non_exhaustive]` attribute for forward-compatible variant additions
- `Io` variant with structured fields: `operation: String`, `path: PathBuf`, `source: std::io::Error`
- `#[error("{operation} at \`{path}\`: {source}")]` for human-readable Display
- `Result<T>` type alias for `std::result::Result<T, AssayError>`

Updated `crates/assay-core/src/lib.rs` with:

- `pub mod error;` declaration (placed before existing modules)
- `pub use error::{AssayError, Result};` re-exports

Added 3 tests:

- `io_error_display_includes_all_context` — verifies exact Display format
- `io_error_source_chain` — verifies `Error::source()` returns the underlying `io::Error`
- `result_alias_works` — verifies `Ok` and `Err` paths through the alias

**Commit:** `199e689` (included in 03-01 task commit)

## Deviations

None — plan executed exactly as written. Implementation was already committed as part of the 03-01 plan execution (commit `199e689`), which included both domain types and error types in a single commit.

## Verification

- `cargo test -p assay-core` — 3 error tests pass
- `cargo check --workspace` — no compilation errors
- `just ready` — full workspace passes (fmt-check + lint + test + deny)
- Display output verified: `"reading config at \`/tmp/config.toml\`: No such file or directory"`
