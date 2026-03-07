---
created: 2026-03-05T00:00
title: save() error paths use directory path instead of temp file path
area: assay-core
provenance: phase-14-review
files:
  - crates/assay-core/src/history/mod.rs
---

## Problem

When `save()` encounters errors during write or rename operations on the temp file, error messages reference the directory path rather than the actual temp file path. This makes debugging harder (callers can't locate the temp file) and may leak less useful information.

## Solution

Update error construction to include the temp file path so errors clearly identify which file operation failed. Example: include both the temp path and final destination in error context.

