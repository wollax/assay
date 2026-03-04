---
phase: 12-fileexists-gate-wiring
plan: 01
subsystem: gate-evaluation
tags: [gate, file-exists, dispatch, serde, schema]
requires: [11-type-system-foundation]
provides: [fileexists-gate-wiring]
affects: [13-gate-run-record]
tech-stack:
  added: []
  patterns: [tuple-match-dispatch]
key-files:
  created: []
  modified:
    - crates/assay-types/src/criterion.rs
    - crates/assay-types/src/gates_spec.rs
    - crates/assay-core/src/gate/mod.rs
    - crates/assay-core/src/spec/mod.rs
    - crates/assay-types/tests/schema_roundtrip.rs
    - crates/assay-types/tests/snapshots/schema_snapshots__criterion-schema.snap
    - crates/assay-types/tests/snapshots/schema_snapshots__gate-criterion-schema.snap
    - crates/assay-types/tests/snapshots/schema_snapshots__gates-spec-schema.snap
    - crates/assay-types/tests/snapshots/schema_snapshots__spec-schema.snap
    - crates/assay-types/tests/snapshots/schema_snapshots__workflow-schema.snap
decisions:
  - cmd takes precedence over path when both are set (simpler than mutual exclusivity validation)
  - evaluate_file_exists implementation unchanged (already correct)
  - path field uses same serde attributes as cmd (skip_serializing_if + default)
metrics:
  duration: ~7 minutes
  completed: 2026-03-04
---

# Phase 12 Plan 01: FileExists Gate Wiring Summary

Wire `GateKind::FileExists` into `evaluate()` dispatch via `path: Option<String>` on Criterion/GateCriterion, with tuple-match dispatch and updated skip logic.

## What Changed

### Type Changes (assay-types)

- **`Criterion`**: Added `path: Option<String>` field between `cmd` and `timeout`, with `#[serde(skip_serializing_if = "Option::is_none", default)]` for backward compatibility.
- **`GateCriterion`**: Same `path` field addition with identical serde attributes.

### Dispatch Changes (assay-core)

- **`evaluate()`**: Replaced 2-arm `match &criterion.cmd` with 3-arm `match (&criterion.cmd, &criterion.path)`:
  - `(Some(cmd), _)` → `evaluate_command` (cmd takes precedence)
  - `(None, Some(path))` → `evaluate_file_exists` (file existence check)
  - `(None, None)` → `evaluate_always_pass` (descriptive criterion)
- **`evaluate_all()`**: Skip condition changed from `cmd.is_none()` to `cmd.is_none() && path.is_none()`.
- **`evaluate_all_gates()`**: Same skip condition update.
- **`to_criterion()`**: Added `path: gc.path.clone()` to propagate field from GateCriterion to Criterion.

### Test Changes

- Added 4 new integration tests proving FileExists dispatch works:
  - `evaluate_dispatches_file_exists_present` — existing file passes through evaluate()
  - `evaluate_dispatches_file_exists_missing` — missing file fails with clear error
  - `evaluate_all_includes_file_exists_criteria` — FileExists not skipped, descriptive-only is
  - `evaluate_cmd_takes_precedence_over_path` — cmd wins when both are set
- Updated ~25 existing struct literals across 5 files to include `path: None`
- Regenerated 5 schema snapshots (criterion, gate-criterion, gates-spec, spec, workflow)

## Decisions Made

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | `cmd` takes precedence over `path` when both set | Simpler than mutual exclusivity; validation warning deferred to future phase |
| 2 | `path` field uses `serde(skip_serializing_if, default)` | Matches `cmd` pattern exactly; preserves backward compat with existing specs |
| 3 | No changes to `evaluate_file_exists` implementation | Already correct — resolves path relative to working_dir, uses Path::exists() |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Missing path: None in 2 additional schema_roundtrip.rs literals**

- **Found during:** Task 2 (schema snapshot regeneration)
- **Issue:** The bulk `replace_all` for `cmd: None, timeout: None` matched 12-space indented literals but missed 4-space and 16-space indented ones in `criterion_without_cmd_validates` and `workflow_validates`
- **Fix:** Added `path: None` to remaining 2 Criterion struct literals manually
- **Files modified:** `crates/assay-types/tests/schema_roundtrip.rs`
- **Commit:** d652491

## Verification Results

- `just ready` passes (fmt-check + lint + test + deny)
- All 4 new FileExists dispatch tests pass
- `cargo insta test -p assay-types` shows no pending snapshots
- `evaluate_file_exists` is called from `evaluate()` at line 65
- No `unreachable` or `todo!` for FileExists in the codebase

## Next Phase Readiness

No blockers. Phase 13 (Gate Run Record) can proceed — it depends on the type system foundation (Phase 11) which is complete, and the FileExists wiring does not affect its scope.
