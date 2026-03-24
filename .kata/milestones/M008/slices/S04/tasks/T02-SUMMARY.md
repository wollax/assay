---
id: T02
parent: S04
milestone: M008
provides:
  - compute_failure_frequency sorted by fail_count desc, then spec_name asc, criterion_name asc
  - compute_milestone_velocity using updated_at - created_at for days_elapsed, sorted by chunks_per_day desc
  - compute_milestone_velocity filters to completed_chunks > 0 (not draft-status based)
  - milestone_scan errors return empty vec instead of propagating (no milestones dir is not fatal)
  - Unit tests for empty results dir and skipped criterion edge cases
key_files:
  - crates/assay-core/src/history/analytics.rs
key_decisions:
  - "Sort failure_frequency by fail_count desc first — surfaces worst offenders at top of report"
  - "Velocity uses updated_at - created_at (not now - created_at) — measures actual milestone timeline, not wall clock since creation"
  - "Filter velocity by completed_chunks.is_empty() rather than status==Draft — simpler, catches all zero-progress milestones regardless of status"
patterns_established:
  - "milestone_scan errors caught and return empty vec — no milestones dir is not fatal, consistent with results dir pattern"
observability_surfaces:
  - AnalyticsReport.unreadable_records counts deserialization failures
  - eprintln warnings for individual unreadable records (spec name + error)
duration: 10min
verification_result: passed
completed_at: 2026-03-24T00:00:00Z
blocker_discovered: false
---

# T02: Implement compute functions

**Refined sort ordering, velocity calculation, and filter logic in analytics compute functions; added 2 unit tests**

## What Happened

T01 had already implemented the core compute functions (it noted this deviation in its summary). T02 aligned the implementation with the task plan's specific contracts:

1. **Sort order for `compute_failure_frequency`**: Changed from spec_name/criterion_name alphabetical to fail_count desc first, then spec_name asc, criterion_name asc. This surfaces worst-offending criteria at the top.

2. **`compute_milestone_velocity` days_elapsed**: Changed from `now - created_at` to `updated_at - created_at`. This measures the actual milestone timeline rather than wall-clock time since creation.

3. **`compute_milestone_velocity` filter**: Simplified from status-based check (`Draft && empty`) to `completed_chunks.is_empty()`. Any milestone with zero completed chunks is excluded, regardless of status.

4. **`compute_milestone_velocity` error handling**: `milestone_scan` errors now return empty vec instead of propagating — no milestones dir is not fatal.

5. **Unit tests**: Added `test_empty_results_returns_empty` and `test_skipped_criterion_not_counted` in `#[cfg(test)]` module within `analytics.rs`.

## Verification

- `cargo test -p assay-core -- analytics` — all 10 tests green (8 integration + 2 unit)
- `cargo check -p assay-core` — compiles clean with no warnings

### Slice-level verification (partial — T02 is intermediate):
- ✅ `cargo test -p assay-core -- analytics` — all pass
- ✅ `crates/assay-core/tests/analytics.rs` — 8 integration tests pass
- ⏳ `cargo test -p assay-cli -- history` — CLI not yet implemented (T03)
- ⏳ `just ready` — deferred to T03 (final task)

## Diagnostics

- Run `cargo test -p assay-core -- analytics --nocapture` to see all analytics tests with stderr output
- `AnalyticsReport.unreadable_records` field indicates skipped data at runtime

## Deviations

None — T01 had pre-implemented the functions; T02 aligned them with the specified contracts.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-core/src/history/analytics.rs` — Refined sort ordering, velocity calculation, filter logic; added #[cfg(test)] unit tests
