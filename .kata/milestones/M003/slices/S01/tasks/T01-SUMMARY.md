---
id: T01
parent: S01
milestone: M003
provides:
  - Two-phase merge_execute() with abort_on_conflict parameter
  - ConflictResolutionConfig type with serde/schemars/inventory
key_files:
  - crates/assay-core/src/merge.rs
  - crates/assay-types/src/orchestrate.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-config-schema.snap
  - crates/assay-core/src/orchestrate/merge_runner.rs
key_decisions:
  - Default model for ConflictResolutionConfig set to claude-sonnet-4-20250514 with 120s timeout
patterns_established:
  - Two-phase merge lifecycle: abort_on_conflict=false leaves working tree conflicted for handler resolution
observability_surfaces:
  - MergeExecuteResult.was_conflict + MERGE_HEAD existence distinguishes aborted vs live conflict state
duration: 15m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Two-phase merge_execute and conflict resolution types

**Added `abort_on_conflict: bool` parameter to `merge_execute()` and `ConflictResolutionConfig` type to assay-types**

## What Happened

Added a new `abort_on_conflict: bool` parameter to `merge_execute()` in `merge.rs`. When `true` (default), behavior is identical to the previous implementation — conflicts are detected, details collected, and `git merge --abort` runs to leave the repo clean. When `false`, the abort is skipped and the working tree remains in a conflicted state with conflict markers in files and `MERGE_HEAD` present, enabling a downstream handler to resolve the conflict.

Updated the single caller in `merge_runner.rs` (line 140) and all three existing test calls to pass `abort_on_conflict: true`, preserving existing behavior.

Created `ConflictResolutionConfig` struct in `assay-types/src/orchestrate.rs` with `enabled: bool`, `model: String` (default `"claude-sonnet-4-20250514"`), and `timeout_secs: u64` (default `120`). Type has `deny_unknown_fields`, `Serialize`/`Deserialize`/`JsonSchema` derives, `Default` impl, and inventory schema submission. Added re-export in `lib.rs`.

Added 4 unit tests for `ConflictResolutionConfig` (default, roundtrip, defaults-applied, deny-unknown-fields) and 2 integration tests for two-phase merge (conflict-leaves-tree-conflicted, abort-on-conflict-true-still-aborts). The integration tests use real git repos with diverging branches.

## Verification

- `cargo test -p assay-core merge` — 31 passed (existing 29 + 2 new two-phase tests)
- `cargo test -p assay-types --all-features --test schema_snapshots` — 53 passed (including new conflict-resolution-config snapshot)
- `cargo test -p assay-types --features orchestrate -- conflict_resolution_config` — 5 passed (4 unit + 1 snapshot)
- `cargo test -p assay-core` — all 683 tests passed
- `cargo check --workspace` — clean build, no warnings

### Slice-level verification status (T01 is task 1 of 4):
- ✅ `cargo test -p assay-core merge_execute_two_phase` — passes (2 new tests)
- ⬜ `cargo test -p assay-core merge_runner_conflict_resolution` — not yet (T03)
- ⬜ `cargo test -p assay-core resolve_conflict` — not yet (T02)
- ✅ `cargo test -p assay-types schema_snapshots` — passes (53 total)
- ⬜ `cargo test -p assay-cli run` — not yet (T04)
- ⬜ `cargo test -p assay-mcp orchestrate_run` — not yet (T04)

## Diagnostics

- Check `was_conflict` on `MergeExecuteResult`. If the caller passed `abort_on_conflict: false`, the working tree is still conflicted — detectable via `MERGE_HEAD` existence in the git dir.
- `ConflictResolutionConfig::default()` produces `enabled: false`, safe no-op configuration.

## Deviations

- Extracted a `setup_conflicting_repo()` helper to share between two-phase tests and the existing conflict test — minor refactor, not a plan deviation.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/merge.rs` — Added `abort_on_conflict` parameter to `merge_execute()`, gated abort block, added 2 integration tests + helper
- `crates/assay-core/src/orchestrate/merge_runner.rs` — Updated `merge_execute()` call to pass `abort_on_conflict: true`
- `crates/assay-types/src/orchestrate.rs` — Added `ConflictResolutionConfig` type with defaults, inventory submission, and 4 unit tests
- `crates/assay-types/src/lib.rs` — Added `ConflictResolutionConfig` to re-exports
- `crates/assay-types/tests/schema_snapshots.rs` — Added `conflict_resolution_config_schema_snapshot` test
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-config-schema.snap` — New schema snapshot
