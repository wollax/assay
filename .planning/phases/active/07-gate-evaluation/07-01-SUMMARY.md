---
phase: 07-gate-evaluation
plan: 01
subsystem: gate-evaluation
tags: [process-execution, timeout, pipe-draining, gate-types, error-handling]
dependency-graph:
  requires: [03-error-types-and-domain-model, 06-spec-files]
  provides: [gate-evaluate, gate-evaluate-all, gate-file-exists, timeout-resolution, gate-run-summary]
  affects: [07-02 (CLI gate run), 08-mcp-server-tools]
tech-stack:
  added: []
  patterns: [spawn-reader-threads-try-wait, process-group-kill, tail-biased-truncation, three-tier-timeout]
key-files:
  created: []
  modified:
    - crates/assay-types/src/gate.rs
    - crates/assay-types/src/criterion.rs
    - crates/assay-core/src/gate/mod.rs
    - crates/assay-core/src/error.rs
    - crates/assay-core/Cargo.toml
    - crates/assay-types/tests/schema_roundtrip.rs
    - schemas/criterion.schema.json
    - schemas/gate-kind.schema.json
    - schemas/gate-result.schema.json
    - schemas/spec.schema.json
    - schemas/workflow.schema.json
decisions:
  - "serde added to assay-core dependencies for GateRunSummary/CriterionResult Serialize derive"
  - "STATE.md decision 'Command::output() for gate execution' superseded by spawn + reader threads + try_wait (needed for timeout)"
  - "Truncation uses str::ceil_char_boundary for safe UTF-8 slicing on tail-biased truncation"
  - "GateRunSummary and CriterionResult live in assay-core::gate (computed summaries, not DTOs)"
  - "evaluate_file_exists is a standalone public function, not derived from Criterion (future phases add file-check criteria)"
  - "Minimum timeout floor of 1 second enforced by resolve_timeout"
metrics:
  duration: "25m"
  completed: 2026-03-02
---

# Phase 7 Plan 1: Gate Evaluation Engine Summary

Sync gate evaluation engine with spawn + reader threads + try_wait timeout, FileExists variant, truncation metadata, and three-tier timeout resolution.

## What Was Done

### Task 1: Extend types and add error variants

**Types changes (assay-types):**
- Added `GateKind::FileExists { path: String }` variant with TOML roundtrip test and schema validation
- Added `truncated: bool` and `original_bytes: Option<u64>` fields to `GateResult` with correct serde skip attributes
- Added `timeout: Option<u64>` field to `Criterion` with serde skip attributes
- Regenerated all JSON schema files and snapshot tests

**Error changes (assay-core):**
- Added `AssayError::GateExecution { cmd, working_dir, source }` for spawn failures
- Added `AssayError::SpecNotFound { name, specs_dir }` for spec lookup failures
- Added Display tests for both new variants

**Dependency wiring:**
- Added `chrono.workspace = true` to assay-core (needed for `Utc::now()` in gate evaluation)
- Added `serde.workspace = true` to assay-core (needed for `Serialize` on summary types)

**Affected files updated:**
- All Criterion struct constructions in spec/mod.rs tests updated with `timeout: None`
- All GateResult constructions in gate.rs and schema_roundtrip.rs tests updated with `truncated: false, original_bytes: None`
- Schema snapshot files updated via `INSTA_UPDATE=always`

### Task 2: Implement gate evaluation engine

**Public API (crates/assay-core/src/gate/mod.rs, 671 lines):**
- `evaluate(criterion, working_dir, timeout) -> Result<GateResult>` — dispatches to Command or AlwaysPass based on criterion.cmd
- `evaluate_all(spec, working_dir, cli_timeout, config_timeout) -> GateRunSummary` — sequential evaluation with pass/fail/skip counting
- `evaluate_file_exists(path, working_dir) -> Result<GateResult>` — file existence check relative to working_dir
- `resolve_timeout(cli, criterion, config) -> Duration` — three-tier precedence with 1s minimum floor

**Internal implementation:**
- `evaluate_command()` — spawns `sh -c <cmd>` with `Stdio::piped()`, `process_group(0)` (Unix), reader threads for deadlock-free pipe draining, `try_wait` polling at 50ms for timeout enforcement, zombie reaping after kill
- `evaluate_always_pass()` — immediate pass result
- `truncate_output()` — tail-biased truncation at 64KB using `str::ceil_char_boundary`

**Summary types:**
- `GateRunSummary` — spec_name, results, passed/failed/skipped counts, total_duration_ms
- `CriterionResult` — criterion_name, Option<GateResult> (None for skipped)

**Tests (16 tests, all passing):**
- `evaluate_echo_hello` — GATE-01, GATE-02: passed=true, stdout="hello\n", exit_code=Some(0)
- `evaluate_failing_command` — failed command with stderr evidence, exit_code=Some(1)
- `evaluate_timeout` — GATE-03: sleep 10 with 1s timeout, passed=false, exit_code=None, stderr contains "timed out"
- `evaluate_always_pass_criterion` — no cmd, AlwaysPass, duration_ms=0
- `evaluate_file_exists_present` — GATE-06: temp file exists, passed=true
- `evaluate_file_exists_missing` — GATE-06: missing file, passed=false, stderr="file not found"
- `evaluate_working_dir_is_respected` — GATE-04: pwd output matches working_dir
- `truncate_output_within_budget` — unchanged, not truncated
- `truncate_output_over_budget` — tail-truncated with indicator
- `resolve_timeout_cli_wins` — CLI overrides all
- `resolve_timeout_criterion_wins_over_config` — per-criterion beats config
- `resolve_timeout_config_used` — config used when no CLI/criterion
- `resolve_timeout_default_300s` — no overrides returns 300s
- `resolve_timeout_minimum_floor` — 0 becomes 1s
- `evaluate_all_mixed_criteria` — GATE-07: mixed pass/fail/skip counts
- `evaluate_all_captures_spawn_failure` — spawn error captured as failed result

## Requirements Delivered

| Requirement | Verification | Status |
|-------------|-------------|--------|
| GATE-01 | `evaluate_echo_hello` test | Done |
| GATE-02 | `evaluate_echo_hello` + `evaluate_failing_command` tests | Done |
| GATE-03 | `evaluate_timeout` test | Done |
| GATE-04 | `evaluate_working_dir_is_respected` test | Done |
| GATE-06 | `evaluate_file_exists_present` + `evaluate_file_exists_missing` tests | Done |
| GATE-07 | `evaluate_all_mixed_criteria` test | Done |
| GATE-08 | Module-level doc comment + function doc comments with spawn_blocking guidance | Done |

## Decisions Made

1. **serde added to assay-core** — needed for `Serialize` derive on `GateRunSummary` and `CriterionResult`. These are computed summaries that need JSON serialization for CLI `--json` output.

2. **Command::output() superseded** — STATE.md previously decided `Command::output()` for gate execution. Research found this blocks with no timeout mechanism. Replaced with `spawn()` + reader threads + `try_wait` polling, which is also deadlock-free AND supports timeout.

3. **evaluate_file_exists is standalone** — not derived from Criterion's cmd field. The `evaluate()` function only produces Command or AlwaysPass from Criterion. FileExists is called directly for file-check gates (future phases will integrate it with criteria).

4. **Minimum timeout floor** — `resolve_timeout()` enforces 1 second minimum. A 0-second timeout would instantly kill every command.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] serde dependency missing from assay-core**
- **Found during:** Task 2
- **Issue:** `GateRunSummary` and `CriterionResult` need `#[derive(Serialize)]` but assay-core didn't depend on serde directly
- **Fix:** Added `serde.workspace = true` to assay-core's `[dependencies]`
- **Files modified:** `crates/assay-core/Cargo.toml`
- **Commit:** f7a281e (included in Task 1 commit alongside chrono)

**2. [Rule 1 - Bug] Schema snapshots and generated schema files stale**
- **Found during:** Task 1 verification
- **Issue:** Adding new fields to GateKind/GateResult/Criterion changed schemas, breaking insta snapshot tests and leaving generated schema files outdated
- **Fix:** Ran `INSTA_UPDATE=always cargo test` to update snapshots, then `cargo run --example generate-schemas` to regenerate schema files
- **Files modified:** 5 snapshot files, 5 schema files
- **Commit:** f7a281e

## Next Phase Readiness

Plan 07-02 (CLI gate run command) can proceed immediately. All gate evaluation primitives are in place:
- `gate::evaluate_all()` provides the aggregate evaluation needed by the CLI
- `gate::resolve_timeout()` handles the `--timeout` flag precedence
- `GateRunSummary` serializes to JSON for `--json` output
- Error variants are ready for CLI error display
