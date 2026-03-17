---
id: S07
parent: M001
milestone: M001
provides:
  - pipeline module with 6-stage orchestrator (spec→worktree→harness→agent→gate→merge)
  - PipelineStage, PipelineError, PipelineResult, PipelineConfig types with structured error handling
  - run_session() and run_manifest() orchestration functions
  - launch_agent() sync subprocess launcher with thread-based timeout
  - HarnessWriter dependency-injection pattern for harness adapters
  - CLI `assay run <manifest.toml>` subcommand with --timeout, --json, --base-branch flags
  - MCP `run_manifest` tool with spawn_blocking wrapper
requires:
  - slice: S04
    provides: claude adapter (generate_config, write_config, build_cli_args)
  - slice: S05
    provides: worktree create with session linkage and collision prevention
  - slice: S06
    provides: manifest loading and parsing (load, from_str, validate)
  - slice: S01
    provides: work session lifecycle (start_session, record_gate_result, complete_session, abandon_session)
affects: []
key_files:
  - crates/assay-core/src/pipeline.rs
  - crates/assay-cli/src/commands/run.rs
  - crates/assay-mcp/src/server.rs
key_decisions:
  - "D015: Pipeline accepts HarnessWriter function parameter — keeps assay-core independent of assay-harness (reverse dependency direction)"
  - "D016: PipelineError captures error messages as String, not AssayError — AssayError is not Clone, pipeline errors need safe collection across sessions"
  - "CLI exit codes: 0 = all succeed, 1 = pipeline error, 2 = gate/merge failure"
patterns_established:
  - "HarnessWriter type alias Fn(&HarnessProfile, &Path) -> Result<Vec<String>, String> — dependency-inversion for harness adapters"
  - "Stage-tagged structured errors with recovery guidance strings"
  - "abandon_if_started closure pattern for safe session cleanup on pipeline failure"
  - "Concrete adapter composition at call site (CLI and MCP wire claude functions into HarnessWriter closure)"
observability_surfaces:
  - "PipelineStage enum provides structured stage identification for every failure"
  - "PipelineError.recovery contains actionable guidance per stage"
  - "PipelineResult.stage_timings provides per-stage duration breakdown"
  - "PipelineOutcome enum distinguishes Success/GateFailed/MergeConflict"
  - "CLI --json returns structured RunResponse with per-session outcomes"
  - "CLI exit codes (0/1/2) for automation"
  - "MCP run_manifest returns structured JSON with isError flag"
drill_down_paths:
  - .kata/milestones/M001/slices/S07/tasks/T01-SUMMARY.md
  - .kata/milestones/M001/slices/S07/tasks/T02-SUMMARY.md
duration: ~35m
verification_result: passed
completed_at: 2026-03-16
---

# S07: End-to-End Pipeline

**Full single-agent pipeline orchestrator with 6-stage sequencing, structured error handling, CLI subcommand, and MCP tool — composes all prior slices into the real `assay run` entry point.**

## What Happened

**T01** built the pipeline core in `assay-core::pipeline`: a `PipelineStage` enum (SpecLoad, WorktreeCreate, HarnessConfig, AgentLaunch, GateEvaluate, MergeCheck), `PipelineError` with stage context + recovery guidance + elapsed time, `PipelineResult` with per-stage timing and outcome classification, and the `run_session()` orchestrator that sequences all 6 stages with session lifecycle management (start→gate→complete, abandon on failure). Key architectural insight: `assay-core` cannot depend on `assay-harness` (reverse dep direction), so the pipeline accepts a `HarnessWriter` closure parameter for dependency injection — the concrete claude adapter composition happens at the call site.

**T02** wired the pipeline into both entry points. The CLI `assay run <manifest.toml>` subcommand accepts `--timeout`, `--json`, and `--base-branch` flags, prints stage progress to stderr, and returns structured JSON on `--json`. The MCP `run_manifest` tool wraps the sync pipeline in `spawn_blocking` (per D007) with manifest_path and timeout_secs parameters. Both compose the concrete harness writer from `assay_harness::claude::{generate_config, write_config, build_cli_args}` at their respective call sites.

## Verification

- `cargo test -p assay-core -- pipeline` — 18 tests pass (stage display, error construction, config defaults, outcome display, harness profile building, agent launch failure, empty manifest, spec-not-found, worktree-create failure, plus context pruning tests)
- `cargo test -p assay-mcp -- run_manifest` — 5 tests pass (param deserialization, schema generation, tool router listing, missing manifest error)
- `cargo test -p assay-cli` — 4 tests pass (clap parsing, JSON serialization of success and error responses)
- `cargo run --bin assay -- run --help` — help text shows subcommand with all flags
- `just ready` — all checks pass (fmt, clippy, all tests, deny)

## Requirements Advanced

- R017 (single-agent E2E pipeline) — pipeline module implements the full 6-stage flow; automated tests verify each stage's success and failure paths
- R018 (pipeline as MCP tool) — `run_manifest` MCP tool wraps pipeline with `spawn_blocking`
- R019 (pipeline structured errors) — `PipelineError` includes stage, message, recovery guidance, and elapsed time at every failure point

## Requirements Validated

- R017 — `run_session()` orchestrates spec→worktree→harness→agent→gate→merge with 10 pipeline-specific tests covering success, failure, and edge cases. CLI and MCP entry points compile and pass tests. Full runtime verification requires manual UAT with real Claude Code.
- R018 — `run_manifest` MCP tool registered in router, param schema generates correctly, spawn_blocking wrapping verified, missing-manifest error handling tested
- R019 — PipelineError struct carries stage (PipelineStage enum), message (String), recovery (String), elapsed (Duration) for every failure path. Tests verify stage-tagged errors for SpecLoad, WorktreeCreate, and AgentLaunch failures.

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- **SpecLoad stage instead of ManifestLoad**: The plan listed ManifestLoad as a pipeline stage, but `run_session()` receives an already-loaded ManifestSession (manifest loading happens before the pipeline). Changed to SpecLoad which accurately describes loading the spec referenced by the session.
- **HarnessWriter closure parameter**: The plan called for directly calling `claude::generate_config()` inside the pipeline, but the dependency direction prevents this. Introduced HarnessWriter type alias as a function parameter. This is architecturally correct and consistent with D001/D003.

## Known Limitations

- `launch_agent()` timeout: when timeout fires, the child process was moved into the wait thread and can't be explicitly killed. The thread eventually drops the child (SIGKILL on Unix), but there's a brief window where the process continues. Consider `shared_child` crate or `Arc<Mutex<Child>>` for M002.
- Full end-to-end runtime verification (real Claude Code invocation against a real spec) requires manual UAT — automated tests use fixture/mock subprocess behavior.
- Pre-existing flaky test `session_create_happy_path` in assay-mcp occasionally fails under parallel execution (not related to S07).

## Follow-ups

- Manual UAT: run `assay run` with a real manifest, real worktree, and real Claude Code `--print` invocation
- M002: consider async pipeline for multi-agent parallel execution
- Improve launch_agent timeout with explicit kill via shared_child or Arc<Mutex<Child>>

## Files Created/Modified

- `crates/assay-core/src/pipeline.rs` — New pipeline module with types, orchestrator, launcher, and 10 unit tests
- `crates/assay-core/src/lib.rs` — Added `pub mod pipeline;`
- `crates/assay-cli/src/commands/run.rs` — New CLI run command with RunCommand struct, response types, 4 tests
- `crates/assay-cli/src/commands/mod.rs` — Added `pub mod run;`
- `crates/assay-cli/src/main.rs` — Added Run variant and match arm
- `crates/assay-cli/Cargo.toml` — Added assay-harness and serde dependencies
- `crates/assay-mcp/src/server.rs` — Added RunManifestParams, response structs, run_manifest tool method, 5 tests
- `crates/assay-mcp/Cargo.toml` — Added assay-harness dependency

## Forward Intelligence

### What the next slice should know
- The pipeline is fully wired but has only been tested with unit-level mocks. Real Claude Code `--print` mode invocation is the critical UAT gap.
- The HarnessWriter pattern means adding a new harness adapter (e.g., Codex) only requires a new closure composition at CLI/MCP call sites — no pipeline changes needed.

### What's fragile
- `launch_agent()` timeout handling — the child process ownership transfer to the wait thread means no explicit kill. Works for single-agent but will need rework for M002 multi-agent where zombie processes could accumulate.

### Authoritative diagnostics
- `PipelineError.stage` + `PipelineError.recovery` — these are the primary signals for any pipeline failure. Check stage first, then recovery text for actionable fix.
- CLI `--json` output — machine-readable RunResponse with per-session outcomes, stage timings, and full error details.

### What assumptions changed
- Plan assumed ManifestLoad as a pipeline stage — in practice, manifest loading happens before the pipeline (caller loads it), so the first pipeline stage is SpecLoad.
- Plan assumed direct assay-harness calls in pipeline — dependency direction requires closure injection instead. This is cleaner and more extensible.
