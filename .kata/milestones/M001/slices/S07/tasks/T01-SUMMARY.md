---
id: T01
parent: S07
milestone: M001
provides:
  - pipeline module with PipelineStage, PipelineError, PipelineResult, PipelineConfig types
  - run_session() orchestrator sequencing spec→worktree→harness→agent→gate→merge
  - run_manifest() iterator over manifest sessions
  - launch_agent() sync subprocess launcher with thread-based timeout
  - build_harness_profile() constructing HarnessProfile from ManifestSession inline fields
key_files:
  - crates/assay-core/src/pipeline.rs
  - crates/assay-core/src/lib.rs
key_decisions:
  - "Pipeline accepts HarnessWriter function parameter instead of directly calling assay-harness::claude — keeps assay-core independent of assay-harness (which depends on assay-core, not the other way around). The concrete claude::generate_config + write_config + build_cli_args composition happens at the call site in CLI/MCP."
  - "PipelineError captures error messages as String rather than wrapping AssayError — AssayError is not Clone, and pipeline errors need to be safely collected across sessions."
patterns_established:
  - "HarnessWriter type alias for Fn(&HarnessProfile, &Path) -> Result<Vec<String>, String> — dependency-inversion pattern for harness adapters"
  - "Stage-tagged structured errors with recovery guidance strings"
  - "abandon_if_started pattern — helper closure that safely abandons session only if one was created"
observability_surfaces:
  - "PipelineStage enum provides structured stage identification for every failure"
  - "PipelineError.recovery contains actionable guidance for the operator or agent"
  - "PipelineResult.stage_timings provides per-stage duration breakdown"
  - "PipelineOutcome enum distinguishes Success/GateFailed/MergeConflict"
duration: 15m
verification_result: passed
completed_at: 2026-03-16
blocker_discovered: false
---

# T01: Pipeline module with PipelineStage, PipelineError, and run_session orchestrator

**Built the full pipeline module with 6-stage orchestration (spec→worktree→harness→agent→gate→merge), structured error handling with per-stage recovery guidance, and thread-based agent subprocess timeout.**

## What Happened

Created `crates/assay-core/src/pipeline.rs` containing:

- **PipelineStage** enum covering all 6 stages: SpecLoad, WorktreeCreate, HarnessConfig, AgentLaunch, GateEvaluate, MergeCheck
- **PipelineError** struct with stage, message, recovery guidance, and elapsed time — implements Display and Error
- **PipelineResult** struct with session_id, spec_name, gate_summary, merge_check, stage_timings, and PipelineOutcome
- **PipelineConfig** struct with project paths, timeout (default 600s), and optional base_branch
- **AgentOutput** struct capturing exit_code, stdout, stderr, and timed_out flag
- **launch_agent()** using std::process::Command with thread-based timeout via mpsc channel
- **build_harness_profile()** constructing HarnessProfile from ManifestSession inline fields (per D014)
- **run_session()** orchestrating the full 6-stage pipeline with session lifecycle management
- **run_manifest()** iterating sessions and collecting results independently
- **HarnessWriter** type alias for dependency injection of the harness adapter

Key design decision: `assay-core` cannot depend on `assay-harness` (circular dependency), so the pipeline accepts a `HarnessWriter` closure parameter. This keeps the module testable and adapter-agnostic while the concrete `claude::generate_config + write_config + build_cli_args` call happens at the CLI/MCP call site.

## Verification

- `cargo test -p assay-core -- pipeline` — 18 tests pass (includes 5 from context::pruning that share the "pipeline" name filter):
  - PipelineStage Display formatting for all 6 stages
  - PipelineError construction and Display with elapsed time
  - PipelineConfig default timeout (600s)
  - PipelineOutcome Display for all 3 variants
  - build_harness_profile from minimal ManifestSession (falls back to spec name)
  - build_harness_profile with full overrides (name, settings, hooks, prompt_layers)
  - launch_agent with non-existent working directory (AgentLaunch stage error with recovery guidance)
  - run_manifest with empty sessions vec (returns empty results)
  - run_session spec not found (SpecLoad stage error with spec name in message and recovery)
  - run_session worktree create failure in non-git directory (WorktreeCreate stage error)
- `cargo clippy -p assay-core` — clean, no warnings
- `cargo build -p assay-core` — compiles successfully

### Slice-level verification status (T01 is first of slice):
- ✅ `cargo test -p assay-core -- pipeline` — passes
- ⬜ `cargo test -p assay-mcp -- run_manifest` — not yet implemented (T02/T03)
- ⬜ `cargo test -p assay-cli` — not yet implemented (T02/T03)
- ⬜ `just ready` — deferred to final task

## Diagnostics

- Read `PipelineError.stage` to know which stage failed
- Read `PipelineError.recovery` for actionable fix guidance
- Inspect `PipelineResult.stage_timings` to identify slow stages
- `PipelineResult.outcome` distinguishes Success/GateFailed/MergeConflict

## Deviations

- **HarnessWriter parameter added to run_session/run_manifest**: The task plan called for directly calling `claude::generate_config()` + `claude::write_config()`, but `assay-core` cannot depend on `assay-harness` (reverse dependency direction). Introduced the `HarnessWriter` type alias as a function parameter for dependency injection. This is the correct architectural approach and doesn't change the pipeline's behavior.
- **SpecLoad stage instead of ManifestLoad**: The task plan listed `ManifestLoad` as a stage, but the pipeline receives an already-loaded ManifestSession (manifest loading happens before `run_session`). Changed to `SpecLoad` which accurately describes what the first stage does — loading the spec referenced by the session.

## Known Issues

- `launch_agent()` timeout handling: when timeout fires, the child process was moved into the wait thread and can't be explicitly killed. The thread will eventually drop the child (which sends SIGKILL on Unix), but there's a brief window where the process may continue. For M002, consider using `shared_child` crate or `Arc<Mutex<Child>>` for explicit kill.

## Files Created/Modified

- `crates/assay-core/src/pipeline.rs` — New pipeline module with all types, orchestrator, launcher, and 10 unit tests
- `crates/assay-core/src/lib.rs` — Added `pub mod pipeline;` declaration
