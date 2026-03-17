---
estimated_steps: 6
estimated_files: 7
---

# T01: Add ConflictResolution Type, ConflictResolutionResult Struct, and Update Schemas

**Slice:** S02 — Audit Trail, Validation & End-to-End
**Milestone:** M003

## Description

Pure additive types task — no function signatures change and no existing callers break. Adds the `ConflictResolution` audit record type and `ConflictFileContent` helper to `assay-types::orchestrate`, extends `MergeReport` with a backward-compatible `resolutions` field, adds `validation_command` to `ConflictResolutionConfig`, defines the `ConflictResolutionResult` struct in `assay-core` (struct only — no changes to `resolve_conflict()` yet), and regenerates the three invalidated schema snapshots.

The `deny_unknown_fields` constraint on `MergeReport` requires both `#[serde(default)]` and `#[serde(skip_serializing_if = "Vec::is_empty")]` on the `resolutions` field to remain backward-compatible with pre-existing persisted reports that don't have the field.

## Steps

1. In `crates/assay-types/src/orchestrate.rs`:
   - Add `ConflictFileContent` struct: `path: String`, `content: String` — with `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]`, `#[serde(deny_unknown_fields)]`
   - Add `ConflictResolution` struct: `session_name: String`, `conflicting_files: Vec<String>`, `original_contents: Vec<ConflictFileContent>`, `resolved_contents: Vec<ConflictFileContent>`, `resolver_stdout: String`, `validation_passed: Option<bool>` — with same derives and `deny_unknown_fields`; `validation_passed` uses `#[serde(default, skip_serializing_if = "Option::is_none")]`
   - Add `resolutions: Vec<ConflictResolution>` to `MergeReport` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]` — this must come AFTER the existing last field to preserve struct construction in tests
   - Add `validation_command: Option<String>` to `ConflictResolutionConfig` with `#[serde(default, skip_serializing_if = "Option::is_none")]` — update `Default::default()` to set it to `None`
   - Add `inventory::submit!` entries for `conflict-file-content` and `conflict-resolution` schemas

2. In `crates/assay-types/src/lib.rs`:
   - Add `pub use orchestrate::{ConflictFileContent, ConflictResolution};` to the orchestrate re-exports

3. In `crates/assay-core/src/orchestrate/conflict_resolver.rs`:
   - Add `use assay_types::ConflictResolution;` (or full path) and `ConflictFileContent` import
   - Define `pub struct ConflictResolutionResult { pub action: ConflictAction, pub audit: Option<ConflictResolution>, pub repo_clean: bool }` — no derive macros needed beyond `Debug`
   - Do NOT change `resolve_conflict()` yet — leave it returning `ConflictAction`

4. In `crates/assay-types/tests/schema_snapshots.rs`:
   - Add `conflict_file_content_schema_snapshot()` and `conflict_resolution_schema_snapshot()` test functions

5. Run `INSTA_UPDATE=always cargo test -p assay-types schema_snapshots` to regenerate all three invalidated snapshots (`merge-report-schema`, `conflict-resolution-config-schema`) and generate the two new ones (`conflict-file-content-schema`, `conflict-resolution-schema`)

6. Run `just ready` to confirm all suites pass

## Must-Haves

- [ ] `ConflictResolution` and `ConflictFileContent` in `assay-types::orchestrate` with `deny_unknown_fields`, full serde/schemars derives, and inventory registration
- [ ] `MergeReport.resolutions` field with both `serde(default)` and `skip_serializing_if = "Vec::is_empty"` — old JSON without the field deserializes correctly
- [ ] `ConflictResolutionConfig.validation_command` field with `serde(default, skip_serializing_if = "Option::is_none")` — existing JSON without the field deserializes correctly
- [ ] `ConflictResolutionResult` struct defined (pub) in `conflict_resolver.rs`
- [ ] All schema snapshots regenerated and locked
- [ ] `just ready` passes (existing tests for `merge_report_deny_unknown_fields` and `conflict_resolution_config_deny_unknown_fields` still pass)

## Verification

- `cargo test -p assay-types schema_snapshots` — all pass including new `conflict-resolution-schema`
- `cargo test -p assay-types` — all existing type tests pass
- `just ready` — fully green

## Observability Impact

- Signals added/changed: New `ConflictResolution` type establishes the schema for audit records that will appear in `MergeReport.resolutions` and be read by `orchestrate_status`
- How a future agent inspects this: `orchestrate_status` response will include `merge_report.resolutions[i]` with `session_name`, `original_contents` (conflict markers), `resolved_contents`, `resolver_stdout`
- Failure state exposed: `validation_passed: false` in a `ConflictResolution` record indicates validation rejected a resolution (populated by T02)

## Inputs

- `crates/assay-types/src/orchestrate.rs` — existing `MergeReport` and `ConflictResolutionConfig` structs; `inventory::submit!` pattern for schema registration
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — existing `ConflictAction` type import; location for `ConflictResolutionResult`
- S01 summary — `ConflictResolutionConfig` has `deny_unknown_fields` (must use `serde(default)` for new fields)

## Expected Output

- `crates/assay-types/src/orchestrate.rs` — updated with `ConflictFileContent`, `ConflictResolution`, extended `MergeReport` and `ConflictResolutionConfig`
- `crates/assay-core/src/orchestrate/conflict_resolver.rs` — `ConflictResolutionResult` struct defined
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-schema.snap` — new locked snapshot
- `crates/assay-types/tests/snapshots/schema_snapshots__merge-report-schema.snap` — regenerated (includes `resolutions` field)
- `crates/assay-types/tests/snapshots/schema_snapshots__conflict-resolution-config-schema.snap` — regenerated (includes `validation_command` field)
