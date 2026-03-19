---
estimated_steps: 6
estimated_files: 3
---

# T01: Pipeline module with PipelineStage, PipelineError, and run_session orchestrator

**Slice:** S07 — End-to-End Pipeline
**Milestone:** M001

## Description

Create the `assay-core::pipeline` module — the orchestrator that composes S04 (claude adapter), S05 (worktree), S06 (manifest), and pre-existing modules (gate, merge, spec, work_session) into a single sequenced pipeline. This is the core of S07 and the capstone of M001.

The pipeline module contains:
- `PipelineStage` enum for structured error context (R019)
- `PipelineError` struct wrapping stage + source error + elapsed + recovery guidance
- `PipelineResult` struct with per-stage outcomes and timing
- `PipelineConfig` struct with project paths and timeout settings
- `run_session()` — sync function executing one ManifestSession through the full pipeline
- `run_manifest()` — iterates RunManifest sessions calling run_session()
- `launch_agent()` — sync subprocess launcher using std::process::Command with thread-based timeout

## Steps

1. **Add `pub mod pipeline;` to `crates/assay-core/src/lib.rs`.**

2. **Define types in `crates/assay-core/src/pipeline.rs`:**
   - `PipelineStage` enum: `ManifestLoad`, `WorktreeCreate`, `HarnessConfig`, `AgentLaunch`, `GateEvaluate`, `MergeCheck`
   - `PipelineError` struct: `stage: PipelineStage`, `message: String`, `recovery: String`, `elapsed: Duration`. Implement `Display` and `Error`. Don't wrap `AssayError` directly (it's not Clone) — capture the error message string.
   - `PipelineResult` struct: session_id, spec_name, gate_summary (Option), merge_check (Option), stage_timings (Vec of stage+duration), final outcome enum (Success/GateFailed/MergeConflict)
   - `PipelineConfig` struct: project_root, assay_dir, specs_dir, worktree_base, timeout_secs (with sensible default of 600s), base_branch (Option)
   - `AgentOutput` struct: exit_code (Option<i32>), stdout (String), stderr (String), timed_out (bool)

3. **Implement `launch_agent()` in the pipeline module:**
   - Takes: cli args (Vec<String>), working_dir (Path), timeout (Duration)
   - Uses `std::process::Command::new("claude")` with `.current_dir(working_dir)` and `.args(&cli_args)`
   - Spawns child, uses a separate thread with `child.wait()` + `recv_timeout` pattern for timeout
   - On timeout: kills child process, returns AgentOutput with timed_out=true
   - On success: captures stdout/stderr, maps exit code
   - Returns `Result<AgentOutput, PipelineError>` — wraps spawn failures as PipelineError at AgentLaunch stage

4. **Implement `run_session()` — the core orchestration function:**
   - Signature: `pub fn run_session(manifest_session: &ManifestSession, config: &PipelineConfig) -> Result<PipelineResult, PipelineError>`
   - Stage sequence with timing:
     1. **SpecLoad**: `spec::load_spec_entry(&manifest_session.spec, &config.specs_dir)` — loads spec for gate evaluation and prompt building
     2. **WorktreeCreate**: `work_session::start_session()` then `worktree::create()` with session_id linkage
     3. **HarnessConfig**: Construct `HarnessProfile` from ManifestSession inline overrides (settings, hooks, prompt_layers) + spec name as profile name. Call `claude::generate_config()` + `claude::write_config()` to worktree path.
     4. **AgentLaunch**: `claude::build_cli_args()` + `launch_agent()` with CWD set to worktree root. On timeout → abandon session. On crash → abandon session.
     5. **GateEvaluate**: `gate::evaluate_all()` with working_dir set to worktree path. Record gate result via `work_session::record_gate_result()`.
     6. **MergeCheck**: `merge::merge_check()` between base branch and worktree branch. On clean merge + gates pass → `complete_session()`. On failure → session stays in GateEvaluated.
   - On any stage failure: abandon the session (if started), return PipelineError with the stage, elapsed time, and recovery guidance.
   - Recovery guidance strings: e.g., "Check that spec 'X' exists in specs directory", "Inspect worktree at <path>", "Claude Code CLI not found — install from https://claude.ai/code", "Agent timed out after Ns — increase timeout or reduce scope".

5. **Implement `run_manifest()`:**
   - Signature: `pub fn run_manifest(manifest: &RunManifest, config: &PipelineConfig) -> Vec<Result<PipelineResult, PipelineError>>`
   - Iterates `manifest.sessions`, calls `run_session()` for each
   - Continues on failure (collects results) — one session's failure doesn't block others (future-proof for M002 multi-session)

6. **Add unit tests covering:**
   - `PipelineStage` Display formatting
   - `PipelineError` construction and Display
   - `PipelineConfig` default timeout
   - `run_session()` failure at SpecLoad stage (spec not found) — verifies PipelineError has correct stage
   - `run_session()` failure at WorktreeCreate stage (collision) — verifies structured error
   - `launch_agent()` with non-existent binary — verifies AgentLaunch stage error with recovery guidance
   - `run_manifest()` with empty sessions vec — returns empty results
   - HarnessProfile construction from ManifestSession — verifies inline overrides are correctly mapped

## Must-Haves

- [ ] `PipelineStage` enum covers all 6 stages
- [ ] `PipelineError` includes stage, message, recovery guidance, and elapsed time
- [ ] `run_session()` calls all upstream APIs in correct sequence: spec load → session start → worktree create → harness config → agent launch → gate evaluate → merge check
- [ ] `launch_agent()` uses std::process::Command (sync, per D007) with thread-based timeout and kill-on-timeout
- [ ] CWD is set to worktree root when spawning claude subprocess (S04 forward intelligence)
- [ ] HarnessProfile constructed from ManifestSession inline fields (per D014), not embedded
- [ ] Session lifecycle follows Created → AgentRunning → GateEvaluated → Completed/Abandoned ordering
- [ ] On pipeline failure after session start, session is abandoned (not left in AgentRunning)
- [ ] Recovery guidance strings are actionable (not generic "an error occurred")

## Verification

- `cargo test -p assay-core -- pipeline` — all unit tests pass
- `cargo clippy -p assay-core` — clean, no warnings
- `cargo build -p assay-core` — compiles successfully

## Observability Impact

- Signals added/changed: `PipelineStage` enum provides structured stage identification. `PipelineError` wraps every failure with stage context + elapsed time + recovery guidance. `PipelineResult` reports per-stage timing.
- How a future agent inspects this: Read `PipelineError.stage` to know which stage failed. Read `PipelineError.recovery` for actionable fix. Inspect `PipelineResult.stage_timings` to identify slow stages.
- Failure state exposed: Session abandoned with reason string on any pipeline failure. PipelineError.recovery tells the agent exactly what to do next.

## Inputs

- `crates/assay-harness/src/claude.rs` — `generate_config()`, `write_config()`, `build_cli_args()` (S04)
- `crates/assay-core/src/worktree.rs` — `create()` with session_id param (S05)
- `crates/assay-core/src/manifest.rs` — `load()` (S06)
- `crates/assay-core/src/work_session.rs` — `start_session()`, `record_gate_result()`, `complete_session()`, `abandon_session()`
- `crates/assay-core/src/gate/mod.rs` — `evaluate_all()`
- `crates/assay-core/src/merge.rs` — `merge_check()`
- `crates/assay-core/src/spec/mod.rs` — `load_spec_entry()`
- S04 forward intelligence: `build_cli_args()` returns relative paths — CWD must be worktree root
- S05 forward intelligence: `create()` takes `session_id: Option<&str>` — pipeline must pass actual session ID
- S06 forward intelligence: ManifestSession overrides are inline, not embedded HarnessProfile
- D007: sync launcher with `std::process::Command`
- D014: construct HarnessProfile from ManifestSession inline fields

## Expected Output

- `crates/assay-core/src/pipeline.rs` — full pipeline module with types, orchestrator, launcher, and tests
- `crates/assay-core/src/lib.rs` — `pub mod pipeline;` added
