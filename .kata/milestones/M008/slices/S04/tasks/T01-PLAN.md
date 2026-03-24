---
estimated_steps: 5
estimated_files: 3
---

# T01: Create analytics module with types and test scaffold

**Slice:** S04 — Gate History Analytics Engine and CLI
**Milestone:** M008

## Description

Establish the `assay-core::history::analytics` submodule with the three analytics types (`AnalyticsReport`, `FailureFrequency`, `MilestoneVelocity`) and a comprehensive integration test file that defines the contract via synthetic history data. The compute functions are stubbed to return empty/default results so the module compiles — T02 fills them in.

## Steps

1. Create `crates/assay-core/src/history/analytics.rs` with:
   - `FailureFrequency` struct: `spec_name: String`, `criterion_name: String`, `fail_count: usize`, `total_runs: usize`, `enforcement: Enforcement` — derive `Debug, Clone, Serialize, Deserialize`
   - `MilestoneVelocity` struct: `milestone_slug: String`, `milestone_name: String`, `chunks_completed: usize`, `total_chunks: usize`, `days_elapsed: f64`, `chunks_per_day: f64` — derive `Debug, Clone, Serialize, Deserialize`
   - `AnalyticsReport` struct: `failure_frequency: Vec<FailureFrequency>`, `milestone_velocity: Vec<MilestoneVelocity>`, `unreadable_records: usize` — derive `Debug, Clone, Serialize, Deserialize`
   - Stub `compute_failure_frequency(assay_dir: &Path) -> Result<(Vec<FailureFrequency>, usize)>` returning `Ok((vec![], 0))`
   - Stub `compute_milestone_velocity(assay_dir: &Path) -> Result<Vec<MilestoneVelocity>>` returning `Ok(vec![])`
   - Stub `compute_analytics(assay_dir: &Path) -> Result<AnalyticsReport>` calling both and composing
2. Add `pub mod analytics;` to `crates/assay-core/src/history/mod.rs`
3. Create `crates/assay-core/tests/analytics.rs` with test helpers:
   - `create_synthetic_record(assay_dir, spec_name, criteria: Vec<(name, passed, enforcement)>)` — builds a `GateRunRecord` and saves it via `history::save()`
   - `create_synthetic_milestone(assay_dir, slug, name, chunks, completed_chunks, created_at, updated_at)` — writes a milestone TOML file
4. Write integration tests (using synthetic data in temp dirs):
   - `test_analytics_empty_results_dir` — no results dir → empty report, zero unreadable
   - `test_failure_frequency_single_spec` — one spec, 3 runs with mixed pass/fail → correct counts
   - `test_failure_frequency_multi_spec` — two specs, criteria with same names across specs → aggregated by (spec, criterion) pair, not conflated
   - `test_failure_frequency_skips_corrupt_records` — one valid JSON, one corrupt file → one record counted, unreadable_records = 1
   - `test_milestone_velocity_basic` — milestone with 3/5 chunks completed, created 10 days ago → chunks_per_day = 0.3
   - `test_milestone_velocity_zero_elapsed` — milestone created and updated same day → uses max(1, days), no division by zero
   - `test_milestone_velocity_skips_draft_milestones` — draft milestone with zero completed chunks excluded from results
   - `test_compute_analytics_composes_both` — verifies `compute_analytics` returns both frequency and velocity data together
5. Verify the module compiles and tests run (some will fail since compute functions are stubs — that's the expected red state)

## Must-Haves

- [ ] `AnalyticsReport`, `FailureFrequency`, `MilestoneVelocity` types exist in `assay_core::history::analytics` with Serialize/Deserialize
- [ ] `pub mod analytics` added to `history/mod.rs`
- [ ] Integration test file `crates/assay-core/tests/analytics.rs` exists with ≥8 test functions
- [ ] Test helpers create synthetic history records via `history::save()` and milestone TOML files
- [ ] Module compiles — `cargo check -p assay-core`

## Verification

- `cargo check -p assay-core` — compiles without errors
- `cargo test -p assay-core -- analytics` — tests compile and run (stub-dependent tests expected to fail)

## Observability Impact

- Signals added/changed: None (types only, no runtime behavior yet)
- How a future agent inspects this: Read `analytics.rs` for type definitions; run `cargo test -p assay-core -- analytics` to see which contract tests still need implementation
- Failure state exposed: None

## Inputs

- `crates/assay-core/src/history/mod.rs` — existing history module to extend with analytics submodule
- `crates/assay-types/src/enforcement.rs` — `Enforcement` enum used in `FailureFrequency`
- `crates/assay-types/src/milestone.rs` — `Milestone` struct fields for velocity calculation
- S04-RESEARCH.md — type shapes, constraints (D118, D001, D007)

## Expected Output

- `crates/assay-core/src/history/analytics.rs` — new file with 3 types + 3 stubbed functions
- `crates/assay-core/src/history/mod.rs` — modified with `pub mod analytics;` line
- `crates/assay-core/tests/analytics.rs` — new file with 8+ contract tests and synthetic data helpers
