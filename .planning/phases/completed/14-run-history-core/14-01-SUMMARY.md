---
phase: 14
plan: "01"
title: "GateRunRecord type and history persistence module"
subsystem: persistence
tags: [serde, json, atomic-write, tempfile, history]
dependency-graph:
  requires: [11, 13]
  provides: ["GateRunRecord type", "history::save/load/list API"]
  affects: [15, 17]
tech-stack:
  added: ["serde_json (assay-core)", "tempfile promoted to regular dep (assay-core)"]
  patterns: ["atomic tempfile-then-rename writes", "per-spec subdirectory layout"]
key-files:
  created:
    - crates/assay-core/src/history/mod.rs
  modified:
    - crates/assay-types/src/gate_run.rs
    - crates/assay-types/src/lib.rs
    - crates/assay-core/Cargo.toml
    - crates/assay-core/src/lib.rs
decisions:
  - "GateRunRecord wraps GateRunSummary via summary field (no field duplication)"
  - "spec_name accessed via record.summary.spec_name (already embedded, not duplicated)"
  - "save() takes assay_dir and derives results path internally"
  - "generate_run_id() is public for caller flexibility"
  - "Existing Io error variant covers all history errors (no new error variants needed)"
metrics:
  duration: "~31 minutes"
  completed: "2026-03-05"
---

# Phase 14 Plan 01: GateRunRecord type and history persistence module Summary

Defined GateRunRecord in assay-types wrapping GateRunSummary with run_id, assay_version, timestamp, and optional working_dir metadata, then built the history module in assay-core with atomic JSON persistence via tempfile-then-rename.

## What Was Done

### Task 1: GateRunRecord type in assay-types

- Added `GateRunRecord` struct to `crates/assay-types/src/gate_run.rs` with `deny_unknown_fields`, `Serialize`, `Deserialize`, `JsonSchema`
- Fields: `run_id` (String), `assay_version` (String), `timestamp` (DateTime<Utc>), `working_dir` (Option<String>), `summary` (GateRunSummary)
- Registered schema via `inventory::submit!` as `gate-run-record`
- Added re-export in `assay_types::lib.rs`

### Task 2: History module in assay-core

- Promoted `tempfile` from dev-dependency to regular dependency
- Added `serde_json` as regular dependency
- Created `crates/assay-core/src/history/mod.rs` with:
  - `generate_run_id()` - produces `YYYYMMDDTHHMMSSZ-xxxxxx` format using `RandomState` for 24-bit entropy suffix
  - `save()` - atomic write via `NamedTempFile::new_in()` + `sync_all()` + `persist()`, auto-creates directories
  - `load()` - deserializes JSON with `deny_unknown_fields` strictness
  - `list()` - returns sorted run IDs, empty vec for nonexistent directories
- Wired up `pub mod history;` in `assay-core/src/lib.rs`
- 7 unit tests: format validation, file creation, directory auto-creation, roundtrip, empty listing, sorted listing, non-clobber

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| GateRunRecord wraps GateRunSummary (not standalone) | Avoids field duplication; existing consumers unchanged |
| spec_name lives inside summary only | Already in GateRunSummary.spec_name; no duplication |
| save() takes assay_dir, not results_dir | Consistent with other assay-core APIs that operate on the .assay root |
| No new error variants | Existing AssayError::Io with operation/path/source covers all cases |
| Used `std::io::Error::other()` for serde_json errors | Clippy io_other_error lint on edition 2024 |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Clippy io_other_error lint**
- **Found during:** Task 2 verification
- **Issue:** `std::io::Error::new(std::io::ErrorKind::Other, e)` triggers clippy lint on Rust 1.93 (edition 2024)
- **Fix:** Changed to `std::io::Error::other(e)`
- **Files modified:** `crates/assay-core/src/history/mod.rs`
- **Commit:** fe25137

**2. [Rule 1 - Bug] rustfmt formatting**
- **Found during:** Task 2 `just ready` verification
- **Issue:** Three formatting deviations from rustfmt style
- **Fix:** Ran `just fmt`
- **Files modified:** `crates/assay-core/src/history/mod.rs`
- **Commit:** fe25137

## Commits

| Hash | Message |
|------|---------|
| 4c293fa | feat(14-01): add GateRunRecord type to assay-types |
| 9223b1d | feat(14-01): add history module with save/load/list persistence |
| fe25137 | style(14-01): fix rustfmt formatting in history module |

## Verification

- `just ready` passes (fmt-check + lint + test + deny)
- `cargo test -p assay-core -- history` runs 7 tests
- `cargo test -p assay-types` passes (21 tests, schema registered)
- All 119 assay-core tests pass
- All 29 schema roundtrip tests pass

## Next Phase Readiness

Phase 14 Plan 02 (if it exists) can proceed. Phase 15 (CLI history) and Phase 17 (MCP history) can consume `history::load()` and `history::list()`. No blockers.
