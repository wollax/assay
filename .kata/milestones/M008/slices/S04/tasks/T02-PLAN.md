---
estimated_steps: 5
estimated_files: 1
---

# T02: Implement compute functions

**Slice:** S04 — Gate History Analytics Engine and CLI
**Milestone:** M008

## Description

Implement the three compute functions in `analytics.rs` that make the contract tests from T01 pass. This is pure data aggregation over existing file-based history records and milestone TOML files — no new I/O patterns, no new dependencies.

## Steps

1. Implement `compute_failure_frequency(assay_dir: &Path) -> Result<(Vec<FailureFrequency>, usize)>`:
   - Check `assay_dir.join("results").is_dir()` — if not, return `Ok((vec![], 0))`
   - Iterate `read_dir("results/")` to get spec directory names
   - For each spec dir, call `history::list(assay_dir, spec_name)` to get run IDs
   - For each run ID, call `history::load(assay_dir, spec_name, run_id)` wrapped in a match — on `Err`, increment unreadable counter and continue
   - For each successful `GateRunRecord`, iterate `summary.results` — aggregate into a `HashMap<(String, String), (usize, usize, Enforcement)>` keyed by `(spec_name, criterion_name)` tracking `(fail_count, total_runs, enforcement)`
   - For `CriterionResult.result = None` (skipped) — do not count as a run
   - For `result = Some(r)` where `r.passed == false` — increment both fail_count and total_runs
   - For `result = Some(r)` where `r.passed == true` — increment total_runs only
   - Convert HashMap to sorted `Vec<FailureFrequency>` (sort by fail_count desc, then spec_name asc, criterion_name asc)
2. Implement `compute_milestone_velocity(assay_dir: &Path) -> Result<Vec<MilestoneVelocity>>`:
   - Call `milestone::milestone_scan(assay_dir)` — on error, return empty vec (no milestones dir is not fatal)
   - Filter to milestones where `completed_chunks.len() > 0`
   - For each: compute `days_elapsed = (updated_at - created_at).num_seconds() as f64 / 86400.0` then `max(1.0, days_elapsed)`
   - Compute `chunks_per_day = completed_chunks.len() as f64 / days_elapsed`
   - Sort by `chunks_per_day` desc, then `milestone_slug` asc
3. Implement `compute_analytics(assay_dir: &Path) -> Result<AnalyticsReport>`:
   - Call both compute functions, compose into `AnalyticsReport`
4. Add unit tests within `analytics.rs` `#[cfg(test)]` module for edge cases:
   - `test_empty_results_returns_empty` — no results dir
   - `test_skipped_criterion_not_counted` — CriterionResult with `result: None` excluded
5. Run all tests to confirm T01 contract tests now pass

## Must-Haves

- [ ] `compute_failure_frequency` correctly aggregates by `(spec_name, criterion_name)` pair across all specs
- [ ] Deserialization errors in `history::load()` are caught, counted (returned as `usize`), and do not abort the report
- [ ] `compute_milestone_velocity` filters out milestones with zero completed chunks
- [ ] Zero elapsed days handled via `max(1.0, days_elapsed)` — no division by zero
- [ ] `compute_analytics` composes both sub-reports with the unreadable_records count
- [ ] All T01 integration tests pass

## Verification

- `cargo test -p assay-core -- analytics` — all tests green (unit + integration)
- `cargo check -p assay-core` — compiles clean

## Observability Impact

- Signals added/changed: `unreadable_records` count in `AnalyticsReport` — lets consumers know data was skipped
- How a future agent inspects this: Call `compute_analytics()` and check `unreadable_records > 0` to detect schema drift or corrupt records
- Failure state exposed: Deserialization failures are counted, not hidden — the count is surfaced in the report

## Inputs

- `crates/assay-core/src/history/analytics.rs` — stubbed types from T01
- `crates/assay-core/tests/analytics.rs` — contract tests from T01 that must now pass
- `crates/assay-core/src/history/mod.rs` — `list()`, `load()` functions for reading history
- `crates/assay-core/src/milestone/mod.rs` — `milestone_scan()` for reading milestones

## Expected Output

- `crates/assay-core/src/history/analytics.rs` — fully implemented compute functions replacing stubs
- All 8+ integration tests from T01 passing
