---
id: T01
parent: S02
milestone: M005
blocker_discovered: false
provides:
  - "`completed_chunks: Vec<String>` field on `Milestone` struct with `#[serde(default, skip_serializing_if = \"Vec::is_empty\")]`"
  - "Schema snapshot `schema_snapshots__milestone-schema.snap` updated to include `completed_chunks` in JSON schema"
  - "`crates/assay-core/tests/cycle.rs` with 10 fully-written integration tests (compile-fail expected until T02)"
  - "All 10 tests cover: cycle_status (None/Draft/InProgress), active_chunk ordering, cycle_advance (pass/fail/all-done), phase transitions (valid/invalid), no-active-milestone guard"
  - "Backward-compatible type extension — all 131 assay-types tests + 5 milestone_io integration tests pass"
requires: []
affects: [T02, T03, T04]
key_files:
  - crates/assay-types/src/milestone.rs
  - crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap
  - crates/assay-core/tests/cycle.rs
  - crates/assay-core/tests/milestone_io.rs
key_decisions:
  - "completed_chunks uses serde default + skip_serializing_if=Vec::is_empty for backward-compatible TOML (empty vec never written to disk)"
  - "cycle.rs tests import public API surface: active_chunk, cycle_advance, cycle_status, milestone_phase_transition, CycleStatus — T02 must export all from assay_core::milestone::cycle"
patterns_established:
  - "Integration tests in cycle.rs use create_passing_spec/create_failing_spec helpers that write real gates.toml files into tempdir/.assay/specs/<slug>/"
  - "make_milestone_with_status helper: clean constructor with explicit completed_chunks: vec![] for test clarity"
drill_down_paths:
  - .kata/milestones/M005/slices/S02/tasks/T01-PLAN.md
duration: 20min
verification_result: pass
completed_at: 2026-03-19T00:00:00Z
---

# T01: Add `completed_chunks` to Milestone and write failing cycle integration tests

**`completed_chunks: Vec<String>` added to `Milestone`; schema snapshot updated; 10 cycle integration tests written (expected compile-fail until T02)**

## What Happened

Added `completed_chunks: Vec<String>` to the `Milestone` struct in `assay-types` with `#[serde(default, skip_serializing_if = "Vec::is_empty")]`. The field sits between `chunks` and `depends_on` for logical grouping. Updated both struct literals in the `#[cfg(test)]` block (full roundtrip + minimal roundtrip) to include `completed_chunks: vec![]`, and added an explicit assertion in `milestone_minimal_toml_roundtrip` that the serialized TOML does not contain `completed_chunks` when the vec is empty.

Regenerated the milestone schema snapshot via `INSTA_UPDATE=always cargo test -p assay-types --features assay-types/orchestrate`. All 131 assay-types tests pass, snapshot updated. The `milestone_io.rs` integration test `make_milestone` helper was also updated to include `completed_chunks: vec![]` to keep the existing 5 tests green.

Created `crates/assay-core/tests/cycle.rs` with all 10 fully-written integration tests. The test logic is complete — each test sets up a real tempdir, creates TOML specs on disk using `create_passing_spec`/`create_failing_spec` helpers, calls the cycle functions, and asserts on observable outcomes. The file currently fails to compile (`unresolved import assay_core::milestone::cycle`) because the `cycle.rs` module doesn't exist yet — this is the expected state for T01, driving T02 implementation.

## Deviations

- S01 branch was already merged into S02 (commit `b079269`) before this session — no merge action required.
- Feature flag `--features assay-types/orchestrate` required for all workspace tests due to pre-existing `manifest.rs` bug (documented in S02-RESEARCH.md).

## Files Created/Modified

- `crates/assay-types/src/milestone.rs` — added `completed_chunks` field + updated two test struct literals + added empty-vec assertion
- `crates/assay-types/tests/snapshots/schema_snapshots__milestone-schema.snap` — regenerated with `completed_chunks` field in JSON schema
- `crates/assay-core/tests/cycle.rs` — new file, 10 integration tests
- `crates/assay-core/tests/milestone_io.rs` — `make_milestone` helper updated with `completed_chunks: vec![]`
