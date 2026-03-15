---
phase: 44-gate-evaluate-context-budgeting
plan: 01
subsystem: assay-types, assay-core
tags: [types, schema, context-budgeting, diff-truncation]

requires: []
provides:
  - DiffTruncation struct in assay-types
  - GateRunRecord.diff_truncation optional field
  - assay_core::context::context_window_for_model re-export
  - assay_core::gate::extract_diff_files helper
affects:
  - crates/assay-types/src/gate_run.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-core/src/context/mod.rs
  - crates/assay-core/src/gate/mod.rs
  - crates/assay-core/src/evaluator.rs
  - crates/assay-core/src/gate/session.rs
  - crates/assay-core/src/history/mod.rs

tech-stack:
  added: []
  patterns:
    - inventory::submit! schema registration for new type
    - serde(default, skip_serializing_if) for backward-compatible optional field
    - pub use re-export to widen visibility from pub(crate) module

key-files:
  created:
    - crates/assay-types/tests/snapshots/schema_snapshots__diff-truncation-schema.snap
  modified:
    - crates/assay-types/src/gate_run.rs
    - crates/assay-types/src/lib.rs
    - crates/assay-core/src/context/mod.rs
    - crates/assay-core/src/gate/mod.rs
    - crates/assay-types/tests/schema_snapshots.rs
    - crates/assay-types/tests/snapshots/schema_snapshots__gate-run-record-schema.snap

decisions:
  - DiffTruncation placed in gate_run.rs (co-located with GateRunRecord, its sole owner)
  - diff_truncation field on GateRunRecord uses serde(default, skip_serializing_if) for backward compat with existing on-disk records (deny_unknown_fields stays)
  - context_window_for_model was already pub; only needed a pub use re-export from context/mod.rs
  - extract_diff_files uses b/ path (destination) as conventional display choice
  - 4 existing GateRunRecord struct initializers updated with diff_truncation: None (session.rs x2, history/mod.rs x2, evaluator.rs)

metrics:
  duration: ~25 minutes
  completed: 2026-03-15
  commits:
    - e25b3c0 feat(44-01): add DiffTruncation type, context_window_for_model re-export, extract_diff_files helper
    - 6c8ee62 style(44-01): apply rustfmt to extract_diff_files tests and server.rs
---

# Phase 44 Plan 01: DiffTruncation Type + Helpers Summary

## Objective

Established the type foundation and public API surface needed by Plan 02 to wire token-aware budgeting into `gate_evaluate`.

## What Was Done

### Task 1: DiffTruncation type and GateRunRecord extension

- Added `DiffTruncation` struct to `crates/assay-types/src/gate_run.rs` with all required derives (Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema) and an `inventory::submit!` block for schema registry registration under `"diff-truncation"`.
- Added optional `diff_truncation: Option<DiffTruncation>` field to `GateRunRecord` with `#[serde(default, skip_serializing_if = "Option::is_none")]` for backward compatibility with existing on-disk records.
- Re-exported `DiffTruncation` from `assay_types` public API (`lib.rs`).
- Re-exported `context_window_for_model` from `assay_core::context` (was already `pub` in `tokens.rs`, only needed a `pub use` in `mod.rs`).
- Added `extract_diff_files` helper to `assay_core::gate::mod` that parses `diff --git a/<path> b/<path>` headers and returns the `b/` destination path for each file.
- Updated 4 existing `GateRunRecord` struct initializers with `diff_truncation: None` to satisfy exhaustive struct construction.

### Task 2: Schema snapshot regeneration and full test suite

- Added `diff_truncation_schema_snapshot` test to `schema_snapshots.rs`.
- Regenerated snapshots with `INSTA_UPDATE=always` — new `diff-truncation-schema` snapshot created, `gate-run-record-schema` snapshot updated to include the new optional field.
- Applied `cargo fmt` to fix formatting in test assertions and pre-existing server.rs lines.
- `just ready` (fmt-check + lint + test + deny) passes with no regressions — 683 tests pass across assay-types and assay-core.

## Tests Added

Five unit tests for `extract_diff_files` in `crates/assay-core/src/gate/mod.rs`:
- `extract_diff_files_empty_diff` — empty input returns empty vec
- `extract_diff_files_single_file` — single diff header returns one path
- `extract_diff_files_multiple_files` — multi-file diff returns all paths in order
- `extract_diff_files_no_headers` — diff content without headers returns empty vec
- `extract_diff_files_path_with_spaces` — handles file paths containing spaces

## Deviations

None. Implementation matched the plan exactly.
