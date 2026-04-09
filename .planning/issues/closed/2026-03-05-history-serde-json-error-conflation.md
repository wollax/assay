---
created: 2026-03-05T00:00
title: serde_json errors wrapped in AssayError::Io conflates deserialization with I/O errors
area: assay-core
provenance: phase-14-review
files:
  - crates/assay-core/src/history/mod.rs
---

## Problem

`serde_json` deserialization errors are wrapped in `AssayError::Io`, conflating corrupt JSON (deserialization failure) with actual file I/O errors. Callers cannot distinguish between a missing file and invalid JSON content, making error handling and recovery logic impossible.

## Solution

Create a dedicated variant in `AssayError` (or use `serde_json::Error` directly) for deserialization failures. Preserve `Io` for filesystem operations only.

