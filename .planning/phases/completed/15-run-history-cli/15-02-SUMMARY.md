---
phase: 15
plan: 02
status: complete
duration: ~7m
one_liner: "CLI history subcommand with table/detail views and gate run persistence"
commits:
  - hash: 73dc66c
    message: "feat(15-02): add gate history subcommand with table/detail views"
  - hash: b0b0074
    message: "feat(15-02): wire history::save() into gate run handlers"
  - hash: 11d3940
    message: "style(15-02): fix formatting and clippy lint"
---

# Phase 15 — Plan 02 Summary: CLI History Command and Gate Run Integration

## What Was Done

### Task 1: History variant and table/detail views
- Added `GateCommand::History` variant with `name`, `run_id`, `--last`, `--json`, `--limit` args
- Implemented `handle_gate_history()` — table view with columns: #, Timestamp, Status, Passed, Failed, Skipped, Req Failed, Adv Failed, Duration
- Implemented `handle_gate_history_detail()` — detail view showing all record fields and per-criterion results
- Added `format_relative_timestamp()` (seconds/minutes/hours or absolute date) and `format_duration_ms()` helpers
- Added `chrono` workspace dependency to assay-cli
- Wired three match arms: `History { run_id: Some(..) }`, `History { last: true }`, `History { .. }` (table)
- Empty history prints "No history for <spec>" and exits 0; unknown spec prints error and exits 1

### Task 2: Wire history::save() into gate run handlers
- Created `save_run_record()` helper to avoid duplication across code paths
- `handle_gate_run()` JSON path: saves GateRunRecord with full GateRunSummary (including per-criterion results from evaluate_all)
- `handle_gate_run()` streaming path: saves with aggregate counters (no per-criterion detail)
- `handle_gate_run_all()` JSON path: saves one record per spec with full fidelity
- `handle_gate_run_all()` streaming path: tracks per-spec counters and saves one record per spec
- Prune messages go to stderr (suppressed in JSON mode); save failures are non-fatal warnings

### Task 3: Integration verification
- Fixed rustfmt formatting in both assay-cli and assay-core (history/mod.rs long line)
- Fixed clippy lint: replaced `ms % 1000 == 0` with `ms.is_multiple_of(1000)`
- `just ready` passes clean (fmt-check + lint + test + deny)

## Deviations

None. Plan executed as specified.

## Verification Results

- `just ready`: all checks pass (fmt-check, clippy, 127+ tests, cargo-deny)
- `cargo check -p assay-cli`: clean at each task boundary

## Decisions Made

- `save_run_record()` helper extracted to centralize record construction and save logic (avoids 4x duplication)
- Streaming mode records have empty `results` vec and zero `total_duration_ms` (no per-criterion timing available in streaming path)
- `handle_gate_run_all()` streaming path tracks per-spec counters via before/after delta on the shared StreamCounters
