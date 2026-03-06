---
created: 2026-03-05T00:00
title: history load() missing error path tests for nonexistent files and invalid JSON
area: assay-core
provenance: phase-14-review
files:
  - crates/assay-core/src/history/mod.rs
---

## Problem

The `load()` function has no test coverage for error paths:
- Reading a nonexistent file
- Reading a file with invalid/corrupt JSON

Only the happy path (valid JSON) is tested. Error handling logic is untested and may silently fail or panic.

## Solution

Add test cases:
1. `test_load_file_not_found` — verify appropriate error returned
2. `test_load_invalid_json` — verify JSON deserialization error handling



## Resolution

Resolved in Phase 19 Plan 02 (2026-03-06). Tests added in the appropriate crate test modules.
