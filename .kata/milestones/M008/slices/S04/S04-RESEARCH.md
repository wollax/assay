# S04: Gate History Analytics Engine and CLI — Research

**Date:** 2026-03-24
**Domain:** Analytics aggregation over gate run history
**Confidence:** HIGH

## Summary

S04 adds an analytics module that aggregates existing gate run history records to produce two reports: **failure frequency** (which criteria fail most across specs) and **milestone velocity** (chunks completed per day). The implementation is straightforward — all data already exists in `.assay/results/<spec>/` as JSON `GateRunRecord` files, and milestones in `.assay/milestones/<slug>.toml` already track `completed_chunks` and timestamps.

The analytics module belongs in `assay-core::history::analytics` (per D118) as a new submodule. The CLI surface is `assay history --analytics` — a new top-level `history` subcommand on the CLI (not `assay gate history`, which is spec-specific). Types (`AnalyticsReport`, `FailureFrequency`, `MilestoneVelocity`) stay in `assay-core`, not `assay-types`, since they are derived views, not persisted contracts.

The data volume is bounded: history records are pruned per-spec (configurable `max_history`), and milestones are single-digit per project. Even scanning all history across all specs will be O(hundreds) of JSON file loads at worst. No performance optimization needed beyond the existing file-per-record pattern.

## Recommendation

Build `assay-core::history::analytics` as a new submodule with two pure functions:
1. `compute_failure_frequency(assay_dir) -> Result<Vec<FailureFrequency>>` — scans all specs' history, aggregates criterion pass/fail counts
2. `compute_milestone_velocity(assay_dir) -> Result<Vec<MilestoneVelocity>>` — reads milestones, computes chunks/day from `created_at`/`updated_at` and `completed_chunks`
3. `compute_analytics(assay_dir) -> Result<AnalyticsReport>` — composes both into a single report

Add `assay history --analytics [--json]` CLI subcommand. The `--json` flag outputs `AnalyticsReport` as JSON; default is structured text.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Listing specs with history | `fs::read_dir(".assay/results/")` directory iteration | Same pattern as `history::list()` — just enumerate spec dirs instead of run files |
| Loading gate run records | `history::load(assay_dir, spec, run_id)` | Battle-tested deserialization with deny_unknown_fields |
| Scanning milestones | `milestone::milestone_scan(assay_dir)` | Returns all milestones sorted alphabetically, already validated |
| Atomic file I/O | Not needed — analytics is read-only | No persistence layer for analytics |

## Existing Code and Patterns

- `crates/assay-core/src/history/mod.rs` — `list(assay_dir, spec_name) -> Vec<String>` returns sorted run IDs; `load(assay_dir, spec, run_id) -> GateRunRecord` loads one record. These are the building blocks for failure frequency aggregation.
- `crates/assay-core/src/milestone/mod.rs` — `milestone_scan(assay_dir) -> Vec<Milestone>` returns all milestones. `Milestone` has `created_at`, `updated_at`, `completed_chunks: Vec<String>`, `chunks: Vec<ChunkRef>`, `status: MilestoneStatus`. These fields drive velocity calculation.
- `crates/assay-core/src/milestone/cycle.rs` — `CycleStatus` struct (D071) lives in assay-core, not assay-types. Our analytics types follow the same pattern (D118).
- `crates/assay-types/src/gate_run.rs` — `GateRunSummary.results: Vec<CriterionResult>` with `criterion_name`, `result: Option<GateResult>` (where `GateResult.passed: bool`), and `enforcement: Enforcement`. This is the data source for failure frequency.
- `crates/assay-types/src/enforcement.rs` — `Enforcement::Required` vs `Enforcement::Advisory` — failure frequency should track both but surface required failures prominently.
- `crates/assay-cli/src/commands/gate.rs` — `handle_gate_history()` is the existing per-spec history CLI handler. The new `assay history --analytics` is a separate top-level command, not under `gate`.
- `crates/assay-cli/src/commands/mod.rs` — Shared CLI helpers: `project_root()`, `assay_dir()`, `colors_enabled()`, `format_count()`, `format_duration_ms()`, `format_relative_timestamp()`.

## Constraints

- **D118:** Analytics types (`AnalyticsReport`, `FailureFrequency`, `MilestoneVelocity`) live in `assay-core::history::analytics`, not `assay-types`. They are derived views, not persisted contracts.
- **D001:** Zero-trait convention — analytics module uses free functions, no trait abstractions.
- **D007:** Sync core — all analytics computation is synchronous. MCP layer (if added) wraps in `spawn_blocking`.
- **deny_unknown_fields on GateRunRecord:** Records from different schema versions will fail to deserialize. Analytics should handle deserialization errors gracefully (skip corrupt/incompatible records with a warning, don't abort the entire report).
- **History directory structure:** `.assay/results/<spec-name>/<run-id>.json`. To enumerate all specs with history, read the `results/` directory entries.
- **No `history` module as a directory yet:** Currently `crates/assay-core/src/history/mod.rs` is a single file. Adding an `analytics` submodule requires converting to `history/mod.rs` + `history/analytics.rs` — but it's already `history/mod.rs` so we just add `history/analytics.rs` as a peer.
- **CLI top-level subcommand:** `assay history` is a new top-level command (like `assay gate`, `assay spec`, `assay milestone`). It does NOT nest under `gate`. The existing `assay gate history` stays untouched.

## Common Pitfalls

- **Deserializing records from older schema versions** — `GateRunRecord` has `deny_unknown_fields`. If schema evolved between versions, old records may fail to deserialize. Wrap `history::load()` calls in a `match` and count/warn on failures rather than aborting. Report the count of unreadable records in the output.
- **Empty history directories** — Specs with zero runs should not cause errors. `history::list()` already returns empty `Vec` for missing dirs. But the `results/` dir itself may not exist if no gates have ever been run. Guard with `results_dir.is_dir()` before iterating.
- **Velocity calculation with zero elapsed time** — A milestone created and completed on the same day has zero elapsed days. Use `max(1, elapsed_days)` or report velocity as "N chunks in <1 day" to avoid division by zero.
- **Milestone velocity only meaningful for InProgress/Verify/Complete milestones** — Draft milestones have no completed chunks. Filter to milestones with `completed_chunks.len() > 0` or `status != Draft`.
- **Duplicate criterion names across specs** — Failure frequency aggregates across specs. Two specs may have criteria named "unit-tests" — they are different criteria in different specs. Aggregate by `(spec_name, criterion_name)` pair, not just `criterion_name`, to avoid conflating unrelated criteria.

## Open Risks

- **No real `.assay/results/` data in the test project** — The current workspace has no history records (empty results dir). All testing must use synthetic data in temp dirs, same as existing `history/mod.rs` tests. Not a blocker, just means UAT requires a real project.
- **Future S05 dependency** — S05 (TUI analytics screen) consumes `compute_analytics()` and the analytics types. The API surface designed here becomes S05's contract. Keep the types simple and the function signature stable.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust | N/A — standard library data aggregation | Not applicable |
| Ratatui | Already used in project (M006/M007) | Installed patterns exist in codebase |

No external skills needed — S04 is pure Rust data aggregation over existing file formats. No new dependencies required.

## Sources

- Codebase exploration: `history/mod.rs`, `milestone/mod.rs`, `milestone/cycle.rs`, `gate_run.rs`, `enforcement.rs`, `milestone.rs` types
- Decisions register: D001, D007, D071, D118
- Boundary map: S04 produces `AnalyticsReport`, `FailureFrequency`, `MilestoneVelocity` types and `compute_analytics()` function; consumed by S05
