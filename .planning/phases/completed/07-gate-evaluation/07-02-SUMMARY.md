---
phase: 07-gate-evaluation
plan: 02
subsystem: cli-gate-run
tags: [cli, gate-run, streaming-display, json-output, ansi-color, no-color]
dependency-graph:
  requires: [07-01]
  provides: [assay-gate-run-cli]
  affects: [09-cli-surface-completion]
tech-stack:
  added: []
  patterns: [stderr-streaming-progress, stdout-summary, manual-criterion-iteration, evaluate-all-json-path]
key-files:
  created: []
  modified:
    - crates/assay-cli/src/main.rs
decisions:
  - "Streaming progress uses eprint!/eprintln! (stderr), summary line uses println! (stdout), JSON uses println! (stdout)"
  - "For streaming path, CLI iterates criteria manually (not via evaluate_all) to show per-criterion 'running' state"
  - "JSON path uses evaluate_all() directly since no streaming needed"
  - "Evidence display: multi-line output indented with 4 spaces, labeled with 'stdout:' / 'stderr:'"
  - "Working dir resolved as project root (satisfies GATE-04 as explicit choice)"
  - "Config timeout extracted from config.gates.default_timeout"
metrics:
  duration: "~8m"
  completed: 2026-03-02
---

# Phase 7 Plan 02: CLI gate run command with streaming display Summary

**One-liner:** `assay gate run <spec>` CLI command with cargo-test-style streaming progress, per-criterion evidence display, JSON output, timeout override, and verbose mode.

## Execution Details

| Field | Value |
|-------|-------|
| Phase | 07-gate-evaluation |
| Plan | 02 |
| Type | execute |
| Duration | ~8 minutes |
| Completed | 2026-03-02 |
| Tasks | 2/2 (1 auto + 1 checkpoint:human-verify) |
| Tests Total (workspace) | 103 |

## What Was Built

### CLI Subcommand (crates/assay-cli/src/main.rs)

| Command | Description |
|---------|-------------|
| `assay gate run <name>` | Run all executable criteria for a spec with streaming display |
| `assay gate run <name> --json` | Output full GateRunSummary as pretty-printed JSON to stdout |
| `assay gate run <name> --verbose` | Show evidence for passing criteria too (default: failures only) |
| `assay gate run <name> --timeout <secs>` | Override per-run timeout for all criteria |

### Streaming Display Path (default)

The CLI iterates criteria manually rather than calling `evaluate_all()`, allowing per-criterion "running" state to be displayed before evaluation completes (cargo-test style).

For each executable criterion:
1. Prints `  <name> ... running` to stderr (overwritable via `\r\x1b[K`)
2. Calls `assay_core::gate::evaluate()` with `resolve_timeout(cli, criterion, config)` precedence
3. Replaces the running line with `  <name> ... ok` (green) or `  <name> ... FAILED` (red)
4. On failure (or `--verbose`), prints evidence indented with 4 spaces, labeled `stdout:` / `stderr:`

Multi-line evidence has each line individually indented with 4 spaces.

Descriptive-only criteria (no `cmd`) are silently counted as skipped during iteration.

### Summary Line

After all criteria, a blank line followed by:
```
Results: N passed, M failed, K skipped (of T total)
```
Printed to stdout (`println!`) for scriptability. Pass count in green, fail count in red (if > 0), skip count in yellow (if > 0). Respects `NO_COLOR` via `colors_enabled()`.

### JSON Output Path (`--json`)

Calls `assay_core::gate::evaluate_all()` directly (no streaming), serializes the returned `GateRunSummary` via `serde_json::to_string_pretty()`, and prints to stdout. Exit code 1 if `summary.failed > 0`, else 0.

### Error Handling

| Scenario | Behavior |
|----------|----------|
| Config not found | `Error: reading config at ...` + exit 1 |
| Spec not found | `Error: spec '<name>' not found` + exit 1 |
| Spec parse/validation error | Prints `AssayError` Display + exit 1 |
| Zero executable criteria | "No executable criteria found" + exit 0 |
| Criterion spawn failure | Shown as FAILED with error message in evidence |
| Truncated output | Evidence includes "[output truncated]" indicator |

### Color Helpers

Added `format_pass(color: bool) -> &'static str` and `format_fail(color: bool) -> &'static str` following the existing `format_criteria_type` pattern.

### NO_COLOR Support

Reuses existing `colors_enabled()` from phase 6 (`std::env::var("NO_COLOR").is_err()`). All ANSI codes suppressed when set.

## Commits

| Hash | Type | Description |
|------|------|-------------|
| `9a9b26d` | feat | Add gate run CLI subcommand with streaming display |

## Deviations from Plan

None — plan executed exactly as written.

## Decisions Made

| Decision | Rationale |
|----------|-----------|
| `eprint!/eprintln!` for per-criterion progress, `println!` for summary | Streaming progress belongs on stderr (scriptable), final summary on stdout (capturable). JSON on stdout. |
| Manual criterion iteration for streaming path | `evaluate_all()` returns only after all criteria finish; iterating manually allows showing "running" state per criterion. |
| `evaluate_all()` for JSON path | No streaming needed; aggregate result is correct input for JSON serialization. |
| 4-space indentation for multi-line evidence | Uniform indentation without a label repeated per line; labeled once with `stdout:` / `stderr:`. |
| Project root as working_dir default | Explicit choice satisfies GATE-04 (no implicit CWD inheritance). Resolves correctly regardless of where the user invokes the CLI. |
| `config.gates.as_ref().map(|g| g.default_timeout)` for config timeout | The `[gates]` table is optional in config; `as_ref()` avoids moving the Option. |

## Verification Results

All 8 end-to-end scenarios passed human verification:

1. **Passing spec:** `echo-hello ... ok`, summary "1 passed, 0 failed, 1 skipped (of 2 total)", exit 0
2. **Failing spec:** `passes ... ok`, `fails ... FAILED` with stderr evidence, exit 1
3. **Timeout (2s):** `slow ... FAILED` with `[timed out after 2s]` in evidence, exit 1
4. **JSON output:** Valid JSON with `spec_name`, `results`, `passed`/`failed`/`skipped` counts, exit 0
5. **Verbose mode:** Evidence shown for passing criteria, exit 0
6. **Nonexistent spec:** `Error: spec 'nonexistent' not found`, exit 1
7. **NO_COLOR:** No ANSI escape codes in output, exit 0
8. **`just ready`:** All checks passed (103 tests)

## Requirements Satisfied

- **GATE-05**: `assay gate run <spec>` CLI command with streaming display, summary table, evidence on failure, `--verbose`, `--json`, `--timeout` flags, exit code semantics

## Key Files

### Modified
- `crates/assay-cli/src/main.rs` (Gate subcommand, GateCommand enum, handle_gate_run function, format_pass/format_fail helpers)

## Key Links Verified

| From | To | Via |
|------|----|-----|
| `crates/assay-cli/src/main.rs` | `crates/assay-core/src/gate/mod.rs` | `assay_core::gate::{evaluate, resolve_timeout}` (streaming path) |
| `crates/assay-cli/src/main.rs` | `crates/assay-core/src/gate/mod.rs` | `assay_core::gate::{evaluate_all, GateRunSummary}` (JSON path) |
| `crates/assay-cli/src/main.rs` | `crates/assay-core/src/spec/mod.rs` | `assay_core::spec::load` |
| `crates/assay-cli/src/main.rs` | `crates/assay-core/src/config/mod.rs` | `assay_core::config::load` |

## Phase 7 Completion

With Plan 02 complete, Phase 7 (Gate Evaluation) is fully done — 2/2 plans:
- Plan 01: Gate evaluation engine (spawn + reader threads + try_wait, timeout resolution, GateRunSummary, GATE-01 through GATE-04, GATE-06 through GATE-08)
- Plan 02: CLI gate run command with streaming display (GATE-05)

All 8 GATE requirements are now satisfied. Phase 8 (MCP Server Tools) can proceed. No blockers or concerns.
