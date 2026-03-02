---
title: "AssayError construction ergonomics and robustness"
area: assay-core
priority: low
source: PR review #24
---

# AssayError Construction Ergonomics

## Problem

1. **No ergonomic constructor** — every call site must write full struct literal with `map_err`. An `IoResultExt` trait with `.io_context(operation, path)` would reduce boilerplate and enforce consistent operation strings
2. **Empty PathBuf produces misleading errors** — `PathBuf::new()` in Io variant produces `"reading config at \`\`: ..."` with empty backticks
3. **`#[non_exhaustive]` missing on Io variant itself** — adding fields to the struct variant later would break downstream pattern matches

## Solution

Add `IoResultExt` trait and/or `AssayError::io()` constructor in Phase 5 when Io errors are first consumed. Consider `#[non_exhaustive]` on the variant itself.
