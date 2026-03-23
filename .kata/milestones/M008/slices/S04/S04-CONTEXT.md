---
id: S04
milestone: M008
status: ready
---

# S04: Gate history analytics engine and CLI — Context

## Goal

Deliver `assay history --analytics` that outputs gate failure frequency and milestone completion velocity as human-readable tables, with --json for machine-readable output, aggregating from existing `.assay/results/` history records.

## Why this Slice

S04 is the data engine that S05 (TUI analytics screen) renders. Without the aggregation logic and CLI surface, the TUI has nothing to display. S04 has no dependencies on S01–S03 — it reads from the existing history module and milestone scan, both of which are stable since M005. Shipping the CLI first means developers get analytics value immediately, even before the TUI screen exists.

## Scope

### In Scope

- New `assay-core::history::analytics` module with `compute_analytics()` free function
- `AnalyticsReport` type containing `failure_frequency: Vec<FailureFrequency>` and `milestone_velocity: Vec<MilestoneVelocity>`
- `FailureFrequency { criterion_name, spec_name, fail_count, total_runs }` — sorted by fail_count descending
- `MilestoneVelocity { milestone_slug, milestone_name, status, chunks_completed, total_chunks, days_elapsed, velocity }` — where velocity = chunks_completed / days_elapsed
- Velocity days_elapsed: from `created_at` to `completed_at` (for Complete milestones) or to now (for in-progress)
- `assay history --analytics` CLI subcommand outputting formatted tables to stdout
- `--json` flag for machine-readable JSON output
- `--limit N` flag (default 100) — last N runs per spec considered for failure frequency
- `--since YYYY-MM-DD` flag — only consider runs after this date
- Unit tests with synthetic history records proving aggregation correctness
- Analytics types in assay-core (D118), not assay-types

### Out of Scope

- TUI rendering of analytics (S05)
- MCP tool for analytics (future — if agents need it)
- Cross-project or cloud-synced analytics
- Chart rendering in CLI (tables only — charts are TUI/S05)
- Per-milestone filtering (show all milestones; filtering is a future concern)
- Export to CSV or other formats

## Constraints

- Analytics types are derived view types in assay-core::history::analytics, not persisted contracts (D118)
- Uses existing `history::list()` and `history::load()` APIs — no changes to history module
- Uses existing `milestone_scan()` for velocity data — no changes to milestone module
- CLI follows D072 pattern — domain errors exit with code 1 via eprintln
- Zero new dependencies — pure aggregation over existing data structures
- Must stay fast: scan + aggregate should complete in <100ms for 100 runs × 10 specs (typical project)

## Integration Points

### Consumes

- `assay-core::history::list(assay_dir, spec_name)` — get run IDs per spec
- `assay-core::history::load(assay_dir, spec_name, run_id)` — load individual `GateRunRecord`
- `assay-core::milestone::milestone_scan(assay_dir)` — load all milestones for velocity
- `assay-core::spec::list_specs(specs_dir)` — discover all spec names to iterate history
- `GateRunRecord.summary.results[].criterion_name` — for failure frequency aggregation
- `GateRunRecord.timestamp` — for --since filtering
- `Milestone.created_at`, `Milestone.updated_at`, `Milestone.status`, `Milestone.chunks`, `Milestone.completed_chunks` — for velocity

### Produces

- `compute_analytics(assay_dir, specs_dir, options) -> Result<AnalyticsReport>` free function in assay-core::history::analytics
- `AnalyticsReport`, `FailureFrequency`, `MilestoneVelocity` types (pub, accessible from assay-tui for S05)
- `AnalyticsOptions { limit: usize, since: Option<DateTime<Utc>> }` for filtering
- `assay history --analytics [--json] [--limit N] [--since YYYY-MM-DD]` CLI subcommand
- Human-readable table output: failure frequency table (criterion | spec | fails | runs) + velocity table (milestone | status | chunks | days | velocity)

## Open Questions

- None — all behavioral decisions captured during discuss.
