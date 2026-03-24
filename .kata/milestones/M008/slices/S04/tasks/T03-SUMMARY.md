---
id: T03
parent: S04
milestone: M008
provides:
  - `assay history analytics` CLI subcommand with structured text output
  - `assay history analytics --json` for machine-readable JSON output
  - Failure frequency table with Spec, Criterion, Fails, Runs, Rate, Enforcement columns
  - Milestone velocity table with Milestone, Chunks, Days, Rate columns
  - Unreadable records footer when count > 0
  - Non-project error handling with exit code 1
key_files:
  - crates/assay-cli/src/commands/history.rs
  - crates/assay-cli/src/commands/mod.rs
  - crates/assay-cli/src/main.rs
key_decisions:
  - "ANSI coloring uses rate thresholds: red >50%, yellow >0%, green 0% — consistent with gate output severity levels"
  - "Velocity rate colored green only when >1.0 chunks/day — avoids noise on slow milestones"
patterns_established:
  - "History subcommand pattern mirrors gate.rs table formatting with COLUMN_GAP and ANSI_COLOR_OVERHEAD"
  - "CLI analytics tests use synthetic AnalyticsReport structs instead of filesystem fixtures for speed and isolation"
observability_surfaces:
  - "`assay history analytics --json` — machine-readable inspection of all analytics data"
  - "Unreadable records count surfaced in text footer and JSON output"
  - "Exit code 1 with descriptive stderr for non-project directories"
duration: 10min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T03: Add `assay history` CLI subcommand

**Wired analytics engine into `assay history analytics` with structured text tables and `--json` output**

## What Happened

Created `crates/assay-cli/src/commands/history.rs` with the `HistoryCommand` enum (Analytics variant with `--json` flag) and `handle()` dispatcher. The text formatter prints two tables — "Gate Failure Frequency" (Spec, Criterion, Fails, Runs, Rate, Enforcement) and "Milestone Velocity" (Milestone, Chunks, Days, Rate) — with ANSI coloring that respects `NO_COLOR` and non-terminal detection. The JSON formatter serializes the full `AnalyticsReport` via `serde_json::to_string_pretty`. Non-project directories get a helpful error message and exit code 1. The `History` variant was added to the `Command` enum in `main.rs` with dispatch and help text examples.

Four tests cover: text output shape validation, JSON round-trip, non-project error guard, and empty project handling.

## Verification

- `cargo test -p assay-cli -- history` — 4/4 tests pass
- `just ready` — all checks pass (fmt, lint, test, deny)
- Manual: `cd /tmp && assay history analytics` → "Error: not an Assay project" with exit code 1
- Slice-level checks:
  - `cargo test -p assay-core -- analytics` — ✅ passes (10 tests)
  - `cargo test -p assay-cli -- history` — ✅ passes (4 tests)
  - `just ready` — ✅ passes

## Diagnostics

- `assay history analytics --json` parses as valid `AnalyticsReport` JSON — future agents can inspect all fields programmatically
- `unreadable_records` field in JSON output indicates data quality issues at runtime
- Non-project detection uses `.assay` directory existence check, consistent with other commands

## Deviations

None — previous attempt had built most of the implementation; this session fixed `cargo fmt` and `clippy::print_literal` lint issues.

## Known Issues

None.

## Files Created/Modified

- `crates/assay-cli/src/commands/history.rs` — New file: HistoryCommand enum, handle(), text/JSON formatters, 4 tests
- `crates/assay-cli/src/commands/mod.rs` — Added `pub mod history`
- `crates/assay-cli/src/main.rs` — Added `History` command variant with dispatch and help text
