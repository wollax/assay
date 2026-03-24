# S04: Gate History Analytics Engine and CLI

**Goal:** Add an analytics module that aggregates gate run history to produce failure frequency and milestone velocity reports, accessible via `assay history --analytics` CLI command.
**Demo:** `assay history --analytics` outputs structured text showing which criteria fail most often (by spec+criterion pair) and milestone completion velocity (chunks/day). `assay history --analytics --json` outputs the same as machine-readable JSON.

## Must-Haves

- `AnalyticsReport`, `FailureFrequency`, `MilestoneVelocity` types in `assay-core::history::analytics` (not assay-types, per D118)
- `compute_failure_frequency()` aggregates across all specs, keyed by `(spec_name, criterion_name)` pair, handles deserialization errors gracefully (skip + count)
- `compute_milestone_velocity()` filters to milestones with `completed_chunks.len() > 0`, uses `max(1, elapsed_days)` for zero-elapsed-time safety
- `compute_analytics()` composes both into a single `AnalyticsReport`
- `assay history --analytics` CLI subcommand produces structured text output
- `assay history --analytics --json` produces JSON output
- Deserialization failures on corrupt/incompatible records are counted and reported, not fatal
- Empty `.assay/results/` directory (or no results dir) produces an empty report, not an error
- All new code has unit tests; integration tests use synthetic history data in temp dirs
- `just ready` passes

## Proof Level

- This slice proves: contract + integration (unit tests with synthetic data, CLI integration via command execution)
- Real runtime required: no (synthetic history records in temp dirs)
- Human/UAT required: yes — real `assay history --analytics` against a project with actual gate run history

## Verification

- `cargo test -p assay-core -- analytics` — unit tests for compute functions (empty data, single spec, multi-spec, corrupt records, velocity edge cases)
- `crates/assay-core/tests/analytics.rs` — integration tests with synthetic history + milestones in temp dirs
- `cargo test -p assay-cli -- history` — CLI subcommand tests (structured text output, JSON output, no-project error)
- `just ready` — full workspace check

## Observability / Diagnostics

- Runtime signals: `eprintln` warnings for unreadable history records (count + spec name), consistent with existing history module pattern
- Inspection surfaces: `assay history --analytics --json` as the machine-readable inspection surface; `AnalyticsReport.unreadable_records` field counts deserialization failures
- Failure visibility: `AnalyticsReport` includes `unreadable_records: usize` so consumers know if data was skipped; empty reports are valid (not errors)
- Redaction constraints: none — analytics data contains no secrets

## Integration Closure

- Upstream surfaces consumed: `history::list()`, `history::load()` from assay-core; `milestone::milestone_scan()` from assay-core; `GateRunRecord`, `CriterionResult`, `Enforcement` from assay-types
- New wiring introduced in this slice: `assay history` top-level CLI subcommand; `assay-core::history::analytics` submodule
- What remains before the milestone is truly usable end-to-end: S05 (TUI analytics screen) consumes `compute_analytics()` and the analytics types for visual rendering

## Tasks

- [x] **T01: Create analytics module with types and test scaffold** `est:30m`
  - Why: Establishes the analytics types, submodule structure, and integration test file with initially-failing tests that define the contract
  - Files: `crates/assay-core/src/history/analytics.rs`, `crates/assay-core/src/history/mod.rs`, `crates/assay-core/tests/analytics.rs`
  - Do: Create `analytics.rs` as a peer of `history/mod.rs`; define `AnalyticsReport`, `FailureFrequency`, `MilestoneVelocity` structs with Serialize/Deserialize; add `pub mod analytics` to `history/mod.rs`; write integration test file with test helpers for creating synthetic history records and milestones; write tests covering: empty results dir, single spec single run, multi-spec aggregation, corrupt record skipping, milestone velocity with zero elapsed days, milestones with no completed chunks filtered out. Stub the three compute functions to return empty/default results so the module compiles.
  - Verify: `cargo test -p assay-core -- analytics` — tests compile but some assertions fail (red state is expected for contract tests)
  - Done when: analytics submodule exists with types, re-exported from `assay_core::history::analytics`, integration test file compiles and runs (some tests may fail)

- [x] **T02: Implement compute functions** `est:45m`
  - Why: Core analytics logic — makes the contract tests from T01 pass
  - Files: `crates/assay-core/src/history/analytics.rs`
  - Do: Implement `compute_failure_frequency(assay_dir) -> Result<(Vec<FailureFrequency>, usize)>` — enumerate `results/` dir entries, for each spec call `history::list()` then `history::load()` wrapping each in a match to count/skip deserialization errors, aggregate by `(spec_name, criterion_name)` tracking fail_count and total_runs, separate required vs advisory. Implement `compute_milestone_velocity(assay_dir) -> Result<Vec<MilestoneVelocity>>` — call `milestone_scan()`, filter to milestones with completed_chunks > 0, compute days_elapsed from created_at to updated_at with `max(1, days)`, compute chunks_per_day. Implement `compute_analytics(assay_dir) -> Result<AnalyticsReport>` composing both. Guard `results/` dir with `is_dir()` check before iterating.
  - Verify: `cargo test -p assay-core -- analytics` — all tests green
  - Done when: All integration tests from T01 pass; `compute_analytics` returns correct aggregated data from synthetic records

- [ ] **T03: Add `assay history` CLI subcommand** `est:45m`
  - Why: Exposes analytics via the CLI — the user-facing surface for R059
  - Files: `crates/assay-cli/src/commands/mod.rs`, `crates/assay-cli/src/commands/history.rs`, `crates/assay-cli/src/main.rs`
  - Do: Create `commands/history.rs` with `HistoryCommand` enum (subcommand `Analytics` with `--json` flag); add `pub mod history` to `commands/mod.rs`; add `History` variant to `Command` enum in `main.rs` with dispatch to `commands::history::handle()`; implement structured text formatter (table-like output for failure frequency sorted by fail_count desc, velocity sorted by chunks_per_day desc); implement JSON output via `serde_json::to_string_pretty`; report unreadable_records count in text output when > 0. Add unit tests for the CLI command (structured text output shape, JSON output shape, no-project error).
  - Verify: `cargo test -p assay-cli -- history` — tests pass; `just ready` — full workspace green
  - Done when: `assay history --analytics` produces structured text; `--json` produces valid JSON; `just ready` passes with zero warnings

## Files Likely Touched

- `crates/assay-core/src/history/mod.rs` — add `pub mod analytics`
- `crates/assay-core/src/history/analytics.rs` — new file: types + compute functions
- `crates/assay-core/tests/analytics.rs` — new file: integration tests with synthetic data
- `crates/assay-cli/src/commands/mod.rs` — add `pub mod history`
- `crates/assay-cli/src/commands/history.rs` — new file: CLI subcommand handler
- `crates/assay-cli/src/main.rs` — add `History` command variant and dispatch
