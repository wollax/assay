---
plan: 61-02
phase: 61-type-correctness-serde-consistency
status: complete
started: "2026-04-09T15:32:32Z"
completed: "2026-04-09T15:50:00Z"
commit: 9b09134
---

# 61-02 Summary: Criterion.when Non-Optional Migration

Change `Criterion.when` from `Option<When>` to `When` with compiler-guided sweep across all sites, plus `When::is_session_end` helper and `nonzero_u32` validation.

## Task Results

| Task | Status | Notes |
|------|--------|-------|
| Task 1: Change Criterion.when to non-optional When | complete | Added is_session_end, nonzero_u32, updated serde attributes |
| Task 2: Update all consumers for non-optional When | complete | 110+ sites swept across 8 files |

## Commits

| Commit | Message |
|--------|---------|
| 9b09134 | fix(61-02): change Criterion.when from Option<When> to When |

## Files Modified

- `crates/assay-types/src/criterion.rs` — `When::is_session_end`, `nonzero_u32`, `Criterion.when: When`
- `crates/assay-types/src/gates_spec.rs` — test literal updates
- `crates/assay-types/tests/schema_roundtrip.rs` — import + literal updates
- `crates/assay-types/tests/snapshots/schema_snapshots__criterion-schema.snap` — updated
- `crates/assay-types/tests/snapshots/schema_snapshots__gate-criterion-schema.snap` — updated
- `crates/assay-types/tests/snapshots/schema_snapshots__gates-spec-schema.snap` — updated
- `crates/assay-types/tests/snapshots/schema_snapshots__spec-schema.snap` — updated
- `crates/assay-types/tests/snapshots/schema_snapshots__when-schema.snap` — updated
- `crates/assay-types/tests/snapshots/schema_snapshots__workflow-schema.snap` — updated
- `crates/assay-core/src/gate/mod.rs` — criterion_matches_phase, tests, doc comment
- `crates/assay-core/src/pipeline_checkpoint.rs` — has_checkpoint_criteria, tests
- `crates/assay-core/src/spec/mod.rs` — test literal updates
- `crates/assay-core/src/spec/validate.rs` — test literal updates
- `crates/assay-core/src/spec/coverage.rs` — test literal updates
- `crates/assay-core/src/evaluator.rs` — test literal updates
- `crates/assay-core/src/review/mod.rs` — test literal updates
- `crates/assay-core/src/wizard.rs` — production code + import
- `crates/assay-mcp/src/server.rs` — test literal updates

## Deviations

- `assay-mcp/src/server.rs` had 3 additional `when: None` sites not listed in the plan (discovered via cargo check --workspace)
- `assay-types/src/gates_spec.rs` and `tests/schema_roundtrip.rs` had additional sites not in plan (discovered via nextest)
- Both deviations auto-fixed per Rule 2 (auto-add missing changes to unblock)

## Verification

- `Option<When>` — zero matches in production code
- `when: None` — zero matches
- `when: Some(...)` — zero matches
- Full test suite: 1153 tests pass (assay-types + assay-core)
- Schema snapshots: 4 updated (criterion, gate-criterion, gates-spec, spec)
