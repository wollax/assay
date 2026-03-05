---
phase: 15
plan: 01
subsystem: history
tags: [pruning, config, save, serde]
dependency_graph:
  requires: [14]
  provides: ["GatesConfig.max_history field", "save() with pruning and SaveResult", "prune() internal function"]
  affects: [15-02]
tech_stack:
  added: []
  patterns: ["Option<usize> with serde(default) for backward-compat on deny_unknown_fields structs"]
key_files:
  created: []
  modified:
    - crates/assay-types/src/lib.rs
    - crates/assay-core/src/history/mod.rs
    - crates/assay-core/src/config/mod.rs
    - crates/assay-mcp/src/server.rs
    - crates/assay-types/tests/schema_roundtrip.rs
    - crates/assay-types/tests/snapshots/schema_snapshots__config-schema.snap
    - crates/assay-types/tests/snapshots/schema_snapshots__gates-config-schema.snap
decisions:
  - "max_history defaults to None (no pruning); CLI will apply default_max_history() (1000) when absent"
  - "Some(0) and None both skip pruning — zero is treated as unlimited"
  - "prune() is private to the history module — only save() calls it"
  - "SaveResult replaces PathBuf as save() return type"
metrics:
  duration: ~17 minutes
  completed: 2026-03-05
---

# Phase 15 Plan 01: Core Layer -- Config Extension, Pruning, and save() Update Summary

**GatesConfig gains max_history: Option<usize> with serde(default); save() returns SaveResult with pruning metadata; prune() removes oldest run files beyond limit.**

## What Was Done

### Task 1: Add max_history field to GatesConfig
- Added `max_history: Option<usize>` to `GatesConfig` with `#[serde(default, skip_serializing_if = "Option::is_none")]`
- Added `default_max_history()` public helper returning 1000 for CLI defaults
- Field uses serde(default) to maintain backward compatibility with deny_unknown_fields

### Task 2: Implement prune() and update save() with SaveResult
- Added `SaveResult` struct with `path: PathBuf` and `pruned: usize` fields
- Added private `prune()` function that lists run IDs, sorts chronologically, deletes oldest beyond limit
- Changed `save()` signature to accept `max_history: Option<usize>` and return `Result<SaveResult>`
- `Some(0)` and `None` both skip pruning; `Some(n)` prunes to keep at most `n` files
- Updated all existing call sites (11 in history tests, 2 in config tests, 2 in MCP tests, 2 in types tests)
- Added 4 new tests: prune_removes_oldest, zero_means_unlimited, none_means_no_pruning, save_result_contains_pruned_count

### Task 3: Update save() call site in gate run handler
- Confirmed no existing `history::save()` call sites in assay-cli — this is a no-op (CLI wiring is Plan 02)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated GatesConfig construction sites across workspace**
- **Found during:** Task 2
- **Issue:** Adding a field to a struct with no Default impl requires updating all construction sites. Tests in assay-core/config, assay-mcp/server, and assay-types/schema_roundtrip used struct literals that became incomplete.
- **Fix:** Added `max_history: None` to all 6 construction sites across 3 crates
- **Files modified:** config/mod.rs, server.rs, schema_roundtrip.rs
- **Commit:** bbf7a67

**2. [Rule 3 - Blocking] Updated insta schema snapshots**
- **Found during:** Verification
- **Issue:** Schema snapshots for GatesConfig and Config were stale after adding the new field
- **Fix:** Ran tests with INSTA_UPDATE=always to regenerate snapshots
- **Files modified:** 2 snapshot files
- **Commit:** 6084b0b

## Verification

- `cargo test`: 212 passed, 3 ignored, 0 failed
- `cargo clippy --all-targets -- -D warnings`: clean
- All 15 history tests pass (11 existing + 4 new prune tests)

## Next Phase Readiness

Plan 15-02 can proceed. It will:
- Wire `history::save()` into gate run handlers, passing `config.gates.as_ref().and_then(|g| g.max_history)`
- Add `assay gate history` CLI subcommand
- The SaveResult return type is ready for CLI display of pruning info
