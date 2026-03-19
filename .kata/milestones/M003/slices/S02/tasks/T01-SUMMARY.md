---
id: T01
parent: S02
milestone: M003
provides:
  - ConflictFileContent and ConflictResolution types in assay-types::orchestrate
  - MergeReport.resolutions backward-compatible field
  - ConflictResolutionConfig.validation_command optional field
  - ConflictResolutionResult struct in assay-core::orchestrate::conflict_resolver
  - Four locked schema snapshots (two new, two regenerated)
key_files:
  - crates/assay-types/src/orchestrate.rs
  - crates/assay-types/src/lib.rs
  - crates/assay-core/src/orchestrate/conflict_resolver.rs
  - crates/assay-types/tests/schema_snapshots.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__conflict-file-content-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__merge-report-schema.snap
  - crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-config-schema.snap
key_decisions:
  - Used serde(default) + skip_serializing_if = "Vec::is_empty" on MergeReport.resolutions to preserve backward compat with deny_unknown_fields
  - ConflictResolutionResult placed before ConflictResolutionOutput in conflict_resolver.rs (struct-only, no logic change)
patterns_established:
  - New optional audit-trail fields on deny_unknown_fields structs must use serde(default) and skip_serializing_if
  - ConflictResolutionResult is the return type shape T02 will use when wiring audit capture into resolve_conflict()
observability_surfaces:
  - MergeReport.resolutions[i] — ConflictResolution audit record (populated by T02) visible via orchestrate_status MCP tool
  - ConflictResolution.validation_passed — None/Some(bool) flag indicating validation outcome per resolution
duration: ~20 min
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Add ConflictResolution Type, ConflictResolutionResult Struct, and Update Schemas

**Added `ConflictFileContent`, `ConflictResolution`, and `ConflictResolutionResult` types plus regenerated four schema snapshots — all existing tests pass with backward-compatible field additions.**

## What Happened

Pure additive types work across three crates:

**assay-types/src/orchestrate.rs:**
- Added `ConflictFileContent { path, content }` with `deny_unknown_fields` and full serde/schemars derives
- Added `ConflictResolution { session_name, conflicting_files, original_contents, resolved_contents, resolver_stdout, validation_passed }` with same derives; `validation_passed` uses `serde(default, skip_serializing_if = "Option::is_none")`
- Extended `MergeReport` with `resolutions: Vec<ConflictResolution>` using `serde(default, skip_serializing_if = "Vec::is_empty")` — old JSON without the field deserializes correctly against `deny_unknown_fields`
- Extended `ConflictResolutionConfig` with `validation_command: Option<String>` using `serde(default, skip_serializing_if = "Option::is_none")` — updated `Default::default()` to set it to `None`
- Added `inventory::submit!` entries for `conflict-file-content` and `conflict-resolution` schemas
- Updated test struct initializers for `MergeReport` and `ConflictResolutionConfig` to include new fields

**assay-types/src/lib.rs:**
- Added `ConflictFileContent` and `ConflictResolution` to the orchestrate re-exports

**assay-core/src/orchestrate/conflict_resolver.rs:**
- Added `ConflictResolution` import from `assay_types`
- Defined `pub struct ConflictResolutionResult { pub action: ConflictAction, pub audit: Option<ConflictResolution>, pub repo_clean: bool }` — struct only, `resolve_conflict()` signature unchanged

**crates/assay-types/tests/schema_snapshots.rs:**
- Added `conflict_file_content_schema_snapshot()` and `conflict_resolution_schema_snapshot()` tests

**Other callers fixed** (struct initializers for new required fields were missing in a struct-exhaustive match context):
- `crates/assay-core/src/orchestrate/merge_runner.rs` — two `MergeReport { ... }` initializers got `resolutions: vec![]`
- `crates/assay-cli/src/commands/run.rs` — one `MergeReport { ... }` initializer got `resolutions: vec![]`
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` test — `ConflictResolutionConfig { ... }` got `validation_command: None`

## Verification

- `INSTA_UPDATE=always cargo test -p assay-types --features orchestrate -- schema_snapshot` — 55 tests pass; 4 snapshots written/updated: `conflict-file-content-schema`, `conflict-resolution-schema`, `merge-report-schema`, `conflict-resolution-config-schema`
- `just ready` — fully green: fmt ✓, lint ✓, 1000+ tests ✓, deny ✓
- `merge_report_deny_unknown_fields` and `conflict_resolution_config_deny_unknown_fields` tests explicitly confirmed passing

## Diagnostics

After T02 populates `MergeReport.resolutions`, the audit trail is inspectable via:
- `orchestrate_status` MCP tool response: `merge_report.resolutions[i].session_name`, `.original_contents`, `.resolved_contents`, `.resolver_stdout`
- `.assay/orchestrator/<run_id>/merge_report.json` on disk: direct JSON inspection
- `ConflictResolution.validation_passed: false` signals a rejected resolution (set by T02)

## Deviations

- `ConflictFileContent` was imported but unused in `conflict_resolver.rs` (it's only referenced inside `ConflictResolution`), so the import was removed to avoid a `#[warn(unused_imports)]` lint warning. `ConflictResolution` alone suffices since `ConflictFileContent` is a field type on it.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-types/src/orchestrate.rs` — added `ConflictFileContent`, `ConflictResolution`, extended `MergeReport` and `ConflictResolutionConfig`, updated test initializers, added inventory entries
- `crates/assay-types/src/lib.rs` — added `ConflictFileContent`, `ConflictResolution` to orchestrate re-exports
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — added `ConflictResolutionResult` struct, fixed import and test initializer
- `crates/assay-core/src/orchestrate/merge_runner.rs` — updated two `MergeReport` initializers with `resolutions: vec![]`
- `crates/assay-cli/src/commands/run.rs` — updated one `MergeReport` initializer with `resolutions: vec![]`
- `crates/assay-types/tests/schema_snapshots.rs` — added two new snapshot tests
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-file-content-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__merge-report-schema.snap` — regenerated (added `resolutions` property)
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-config-schema.snap` — regenerated (added `validation_command` property)
