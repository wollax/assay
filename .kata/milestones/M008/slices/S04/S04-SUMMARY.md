---
id: S04
parent: M008
milestone: M008
provides:
  - AnalyticsReport, FailureFrequency, MilestoneVelocity types in assay-core::history::analytics
  - compute_failure_frequency() aggregating gate results by (spec_name, criterion_name) pair
  - compute_milestone_velocity() with zero-elapsed-day safety and draft-milestone filtering
  - compute_analytics() composing both into a single report
  - `assay history analytics` CLI subcommand with structured text tables
  - `assay history analytics --json` for machine-readable JSON output
  - Unreadable record counting and reporting (not fatal)
requires:
  - slice: none
    provides: standalone slice — consumes existing history::list/load and milestone_scan APIs
affects:
  - S05
key_files:
  - crates/assay-core/src/history/analytics.rs
  - crates/assay-core/tests/analytics.rs
  - crates/assay-cli/src/commands/history.rs
key_decisions:
  - "D118: Analytics types live in assay-core::history::analytics, not assay-types — derived view types, not persisted contracts"
  - "Failure frequency sorted by fail_count desc first — surfaces worst offenders at top"
  - "Velocity uses updated_at - created_at, not now - created_at — measures actual milestone timeline"
  - "Filter velocity by completed_chunks.is_empty() rather than status==Draft — simpler, catches all zero-progress milestones"
  - "ANSI coloring uses rate thresholds: red >50%, yellow >0%, green 0% — consistent with gate output severity"
patterns_established:
  - "Analytics compute functions take &Path (assay_dir) and return Result — consistent with existing history module"
  - "Unreadable records counted and logged to stderr, not fatal — consistent with existing history eprintln pattern"
  - "CLI analytics tests use synthetic AnalyticsReport structs instead of filesystem fixtures"
  - "History subcommand mirrors gate.rs table formatting with COLUMN_GAP and ANSI_COLOR_OVERHEAD"
observability_surfaces:
  - "`assay history analytics --json` — machine-readable inspection of all analytics data"
  - "AnalyticsReport.unreadable_records field exposes data-skip count to consumers"
  - "eprintln warnings for individual unreadable records (spec name + error)"
  - "Exit code 1 with descriptive stderr for non-project directories"
drill_down_paths:
  - .kata/milestones/M008/slices/S04/tasks/T01-SUMMARY.md
  - .kata/milestones/M008/slices/S04/tasks/T02-SUMMARY.md
  - .kata/milestones/M008/slices/S04/tasks/T03-SUMMARY.md
duration: 32min
verification_result: passed
completed_at: 2026-03-24T12:15:00Z
---

# S04: Gate History Analytics Engine and CLI

**Gate failure frequency and milestone velocity analytics engine with structured text and JSON CLI output**

## What Happened

Built the analytics engine in three tasks. T01 created the `assay_core::history::analytics` submodule with three types (`FailureFrequency`, `MilestoneVelocity`, `AnalyticsReport`) and the full compute functions (deviated from plan by skipping stubs — the aggregation logic was straightforward). T02 refined sort ordering (fail_count desc), velocity calculation (updated_at - created_at), and filter logic (completed_chunks.is_empty()). T03 wired everything into `assay history analytics` CLI with ANSI-colored text tables and `--json` output.

The analytics engine scans `.assay/results/` for gate run history and `.assay/milestones/` for velocity data. Corrupt/incompatible records are counted and reported but never fatal. Empty directories produce empty reports, not errors.

## Verification

- `cargo test -p assay-core --test analytics` — 8 integration tests pass (empty dir, single spec, multi-spec, corrupt records, velocity basic, zero-elapsed, draft exclusion, composite report)
- `cargo test -p assay-core -- analytics` — 10 tests pass (8 integration + 2 unit)
- `cargo test -p assay-cli -- history` — 4 tests pass (text output shape, JSON round-trip, no-project error, empty project)
- `just ready` — all checks pass (fmt, lint, test, deny)
- Manual: `cd /tmp && assay history analytics` → "Error: not an Assay project" with exit code 1

## Requirements Advanced

- R059 (Gate history analytics) — CLI surface now complete: `assay history analytics` outputs failure frequency and milestone velocity; `--json` provides machine-readable output. TUI analytics screen remains for S05.

## Requirements Validated

- None fully validated by this slice alone — R059 requires S05 (TUI analytics screen) to be complete.

## New Requirements Surfaced

- None.

## Requirements Invalidated or Re-scoped

- None.

## Deviations

T01 implemented real compute functions instead of stubs (plan called for stubs with failing tests). Since the logic is straightforward aggregation and T02 would implement the identical code, this saved a task's worth of redundant work. T02 then refined the implementation rather than writing it from scratch.

## Known Limitations

- Analytics data is read-only from existing history records — no new storage format or indexing
- ANSI coloring not configurable beyond `NO_COLOR` env var
- No filtering by date range or milestone — reports aggregate all available data

## Follow-ups

- S05 consumes `compute_analytics()` and the analytics types for TUI rendering
- Future: date-range filtering for analytics queries

## Files Created/Modified

- `crates/assay-core/src/history/analytics.rs` — Analytics types and compute functions
- `crates/assay-core/src/history/mod.rs` — Added `pub mod analytics`
- `crates/assay-core/tests/analytics.rs` — 8 integration tests with synthetic data helpers
- `crates/assay-cli/src/commands/history.rs` — CLI subcommand handler with text/JSON formatters
- `crates/assay-cli/src/commands/mod.rs` — Added `pub mod history`
- `crates/assay-cli/src/main.rs` — Added `History` command variant with dispatch

## Forward Intelligence

### What the next slice should know
- `compute_analytics(&assay_dir)` returns `Result<AnalyticsReport>` — the single entry point for all analytics data
- `AnalyticsReport` has `failure_frequency: Vec<FailureFrequency>`, `milestone_velocity: Vec<MilestoneVelocity>`, `unreadable_records: usize`
- `FailureFrequency` has `spec_name`, `criterion_name`, `fail_count`, `total_runs`, `enforcement` (Required/Advisory)
- `MilestoneVelocity` has `milestone_slug`, `chunks_completed`, `days_elapsed`, `chunks_per_day`
- All types derive Serialize/Deserialize — can be passed directly to TUI rendering

### What's fragile
- `compute_failure_frequency` assumes `.assay/results/<spec>/` directory layout — if history storage changes, this breaks

### Authoritative diagnostics
- `assay history analytics --json` is the machine-readable inspection surface — parse it to verify analytics correctness
- `unreadable_records` field is the canary for data quality issues

### What assumptions changed
- Plan assumed T01 would produce stubs and T02 would fill them — T01 produced the real implementation, T02 refined it
