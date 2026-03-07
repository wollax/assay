---
created: 2026-03-05T00:00
title: save() returning PathBuf may leak implementation detail
area: assay-core
provenance: phase-14-review
files:
  - crates/assay-core/src/history/mod.rs
---

## Problem

The `save()` function returns `PathBuf` (the path to the saved file). While this seems useful, it exposes the internal implementation detail of where records are stored. Callers may rely on the path format, making future refactorings (e.g., changing directory structure) harder.

## Solution

Consider returning `()` instead. If callers need the path, they can construct it themselves using the run ID, or `save()` can return an opaque `RunId` handle instead. Evaluate whether the return value is actually used by callers.

