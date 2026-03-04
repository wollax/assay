---
phase: 11-type-system-foundation
plan: 01
subsystem: types
tags: [serde, json-schema, type-relocation, assay-types]
dependency-graph:
  requires: []
  provides: [gate-run-types-in-assay-types, serde-hygiene, schema-registry-entries]
  affects: [11-02, 12-gate-run-record, 13-mcp-tools]
tech-stack:
  added: []
  patterns: [serde-skip-serializing-if-default, inventory-submit-schema-registry]
key-files:
  created:
    - crates/assay-types/src/gate_run.rs
    - schemas/gate-run-summary.schema.json
    - schemas/criterion-result.schema.json
  modified:
    - crates/assay-types/src/lib.rs
    - crates/assay-core/src/gate/mod.rs
    - crates/assay-mcp/src/server.rs
    - schemas/review.schema.json
    - schemas/workflow.schema.json
    - crates/assay-types/tests/snapshots/schema_snapshots__review-schema.snap
    - crates/assay-types/tests/snapshots/schema_snapshots__workflow-schema.snap
decisions:
  - Clean break: no re-exports from assay-core, all consumers import from assay_types
  - Output types (GateRunSummary, CriterionResult) do NOT use deny_unknown_fields
  - Schema registry entries added for both relocated types
metrics:
  duration: ~5 minutes
  completed: 2026-03-04
---

# Phase 11 Plan 01: Relocate Gate Run Types & Serde Hygiene Summary

**One-liner:** Relocated GateRunSummary/CriterionResult to assay-types with Deserialize+JsonSchema derives and enforced skip_serializing_if+default on all Vec fields across Review and Workflow.

## What Was Done

### Task 1: Create gate_run.rs and relocate types
- Created `crates/assay-types/src/gate_run.rs` with `GateRunSummary` and `CriterionResult`
- Added `Deserialize` and `JsonSchema` derives (previously only had `Serialize`)
- Added serde hygiene: `skip_serializing_if = "Vec::is_empty"` on `results`, `skip_serializing_if = "Option::is_none"` on `result`
- Registered both types in the schema registry via `inventory::submit!`
- Removed struct definitions from `assay-core::gate::mod.rs`
- Removed now-unused `use serde::Serialize` from `assay-core::gate`
- Updated `assay-mcp::server` to import types from `assay_types` instead of `assay_core::gate`
- Integrated all uncommitted type work (feature_spec.rs, gates_spec.rs, schemas, and related modifications)

### Task 2: Serde hygiene on Review and Workflow
- Added `#[serde(default, skip_serializing_if = "Vec::is_empty")]` to `Review.comments`
- Added `#[serde(default, skip_serializing_if = "Vec::is_empty")]` to `Workflow.specs` and `Workflow.gates`
- Regenerated all JSON schema files (14 total, including 2 new: gate-run-summary, criterion-result)
- Updated insta snapshots for Review and Workflow schema changes

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| No re-exports from assay-core | Clean break strategy — consumers update imports directly |
| No deny_unknown_fields on gate run types | These are output types, not user-authored configs |
| Integrated all uncommitted type files in Task 1 | Single coherent delivery as specified in phase context |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Schema regeneration**
- **Found during:** Task 2
- **Issue:** Changing serde attributes on Review/Workflow and adding gate_run types required regenerating JSON schema files and updating insta snapshots
- **Fix:** Ran `just schemas` and `cargo insta accept` to update all generated artifacts
- **Files modified:** schemas/*.schema.json, crates/assay-types/tests/snapshots/*.snap

## Verification

- `cargo check --workspace` passes
- `just ready` passes (fmt-check + lint + test + deny)
- 164 tests pass, 3 ignored
- No references to `assay_core::gate::GateRunSummary` or `assay_core::gate::CriterionResult` anywhere in crates/
- All consumers import from `assay_types`

## Commits

| Hash | Description |
|------|-------------|
| `b856b0f` | feat(11-01): relocate GateRunSummary/CriterionResult to assay-types |
| `b3defb3` | fix(11-01): serde hygiene on Review and Workflow Vec fields |

## Next Phase Readiness

- **11-02 (GateRunRecord + persistence types):** Unblocked. GateRunSummary is now in assay-types with full serde support, ready for GateRunRecord to wrap it.
- **No blockers or concerns identified.**
