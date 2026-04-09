---
phase: 61-type-correctness-serde-consistency
plan: 01
subsystem: types
tags: [serde, types, rename, review, checkpoint, snapshot]
requires:
  - "60-process-safety (gate/mod.rs evaluate_checkpoint introduced)"
provides:
  - "CheckpointPhase canonical type name"
  - "OnEvent variant with at_event serde alias"
  - "debug_assert for SessionEnd misuse in evaluate_checkpoint"
affects:
  - "61-02 (subsequent plans in this phase)"
  - "Any future consumer of assay_types::review"
tech-stack:
  added: []
  patterns:
    - "serde alias for backward-compat variant rename"
    - "debug_assert to enforce caller contract"
key-files:
  created:
    - crates/assay-types/tests/snapshots/schema_snapshots__checkpoint-phase-schema.snap
  modified:
    - crates/assay-types/src/review.rs
    - crates/assay-core/src/gate/mod.rs
    - crates/assay-core/src/pipeline_checkpoint.rs
    - crates/assay-core/src/review/mod.rs
    - crates/assay-cli/src/commands/spec.rs
    - crates/assay-types/tests/schema_snapshots.rs
    - crates/assay-types/tests/snapshots/schema_snapshots__gate-diagnostic-schema.snap
    - crates/assay-types/tests/snapshots/schema_snapshots__run-manifest-schema.snap
decisions:
  - "Deleted AtEvent variant rather than aliasing; OnEvent with #[serde(alias = \"at_event\")] provides backward compat"
  - "debug_assert fires for SessionEnd in evaluate_checkpoint — callers must use evaluate_all_with_events instead"
  - "Replaced evaluate_checkpoint(SessionEnd) in empty_criteria_list test with AtToolCall(n=1) to avoid triggering assert"
  - "Added criterion_matches_phase_returns_false_for_session_end test to verify the invariant directly"
metrics:
  duration: "54 minutes"
  completed: "2026-04-09"
---

# Phase 61 Plan 01: Rename SessionPhase to CheckpointPhase Summary

Renamed `review::SessionPhase` to `review::CheckpointPhase`, merged `AtEvent` into `OnEvent` with a `#[serde(alias = "at_event")]` backward-compat tag, and added a `debug_assert!` + doc comment to `evaluate_checkpoint` documenting the SessionEnd no-op contract.

## Tasks Completed

| Task | Commit | Description |
|------|--------|-------------|
| Task 1 | 4466335 | Rename SessionPhase → CheckpointPhase, merge AtEvent → OnEvent, update all consumers |
| Task 2 | 6d7275a | Update schema snapshots for CheckpointPhase rename |

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `empty_criteria_list_returns_empty_summary` test called `evaluate_checkpoint` with `SessionEnd`**

- **Found during:** Task 2 (adding `debug_assert!`)
- **Issue:** The test at line 3284 passed `CheckpointPhase::SessionEnd` to `evaluate_checkpoint`, which would panic in debug builds after the `debug_assert!` was added.
- **Fix:** Changed the test to use `CheckpointPhase::AtToolCall { n: 1 }` (a valid phase). Added a separate `criterion_matches_phase_returns_false_for_session_end` test that verifies the invariant directly through `criterion_matches_phase` rather than through `evaluate_checkpoint`.
- **Files modified:** `crates/assay-core/src/gate/mod.rs`
- **Commit:** 4466335

## Verification Results

- `cargo check --workspace` — clean (2 pre-existing unrelated warnings)
- `cargo nextest run -p assay-types -p assay-core` — 1150 passed, 0 failed
- `grep -rn "SessionPhase" crates/` — no references remain outside `work_session`/`assay-mcp`/`error.rs` (workflow type, unrelated)
- `grep "debug_assert" crates/assay-core/src/gate/mod.rs` — debug_assert present at line 1070
