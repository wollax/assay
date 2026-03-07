# AssayError::Io used for JSON deserialization errors in history::load()

**Area:** core/history
**Severity:** suggestion
**Source:** Phase 15 PR review

## Description

In `history::load()`, a `serde_json` deserialization error is wrapped as `AssayError::Io` with `ErrorKind::InvalidData`. This conflates I/O errors with deserialization errors. A dedicated `AssayError::HistoryParse` variant would let callers distinguish "file missing" from "file corrupt."

Note: This overlaps with existing issue `2026-03-05-history-serde-json-error-conflation.md`.

**File:** `crates/assay-core/src/history/mod.rs`
