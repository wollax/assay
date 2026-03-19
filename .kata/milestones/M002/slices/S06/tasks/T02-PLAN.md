---
estimated_steps: 5
estimated_files: 1
---

# T02: Wire CLI multi-session routing with orchestrator and merge

**Slice:** S06 — MCP Tools & End-to-End Integration
**Milestone:** M002

## Description

Modify `assay run` CLI to detect multi-session manifests and route them through the orchestrator + merge pipeline instead of the sequential `run_manifest()`. Single-session manifests continue using the existing path unchanged. Adds `--failure-policy` and `--merge-strategy` flags. This advances R020 by providing the user-facing entry point for multi-agent orchestration.

## Steps

1. **Add CLI flags** to `RunCommand`:
   - `--failure-policy <skip-dependents|abort>` with default skip-dependents (maps to `FailurePolicy` enum)
   - `--merge-strategy <completion-time|file-overlap>` with default completion-time (maps to `MergeStrategy` enum)

2. **Add multi-session detection** in `execute()`:
   - After loading manifest, check `manifest.sessions.len() > 1 || manifest.sessions.iter().any(|s| !s.depends_on.is_empty())`
   - If single-session (and no depends_on): use existing `run_manifest()` path — no changes
   - If multi-session: branch to new orchestration path

3. **Implement orchestration path**:
   - Build `OrchestratorConfig { max_concurrency: 8, failure_policy }` from CLI flags
   - Construct session runner closure: for each session, call `build_harness_profile()`, optionally `inject_scope_layer()` for sessions with file_scope, then call adapter functions directly (D035) and compose with `run_session()` equivalent — use `assay_core::pipeline::setup_session()` + `assay_core::pipeline::execute_session()`
   - Call `run_orchestrated(&manifest, config, &pipeline_config, &session_runner)`
   - After execution: detect base branch (`git rev-parse --abbrev-ref HEAD` or from `--base-branch` flag), `git checkout <base>` if needed
   - Call `extract_completed_sessions()` on outcomes, then `merge_completed_sessions()` with `MergeRunnerConfig { strategy, project_root, base_branch }` and `default_conflict_handler()`

4. **Format orchestrated results**:
   - Add JSON response types: `OrchestrationResponse` with run_id, per-session outcomes, merge report
   - Human-readable output: per-session status lines, merge summary
   - Exit codes: 0 = all succeed + merge clean, 1 = any error/skip, 2 = merge conflicts

5. **Add/update tests**:
   - Existing `run_command_parses_minimal` / `run_command_parses_all_flags` still pass
   - New test: `run_command_parses_orchestration_flags` with --failure-policy and --merge-strategy
   - New test: multi-session detection helper function (pure logic, testable without filesystem)
   - New test: orchestrated response serializes to JSON

## Must-Haves

- [ ] Single-session manifests use existing `run_manifest()` path unchanged
- [ ] Multi-session manifests route to `run_orchestrated()` + `merge_completed_sessions()`
- [ ] `--failure-policy` and `--merge-strategy` flags parse correctly
- [ ] Session runner closure is `Sync` (uses plain function calls, not `dyn Fn` HarnessWriter)
- [ ] Base branch checkout between execution and merge phases
- [ ] Existing CLI tests still pass
- [ ] 3+ new tests for flag parsing, detection logic, and response serialization

## Verification

- `cargo test -p assay-cli -- run` — all existing + new tests pass
- `cargo clippy -p assay-cli -- -D warnings` — clean

## Observability Impact

- Signals added/changed: CLI stderr output includes orchestration phase (execution → merge), per-session status with timing, merge report summary
- How a future agent inspects this: `assay run manifest.toml --json` returns structured response with orchestration + merge phases
- Failure state exposed: exit code 1 for errors, exit code 2 for merge conflicts; JSON response includes per-session error details

## Inputs

- `crates/assay-cli/src/commands/run.rs` — existing `execute()` function, `RunCommand` struct, response types
- `crates/assay-core/src/orchestrate/executor.rs` — `run_orchestrated()` signature, `OrchestratorConfig`
- `crates/assay-core/src/orchestrate/merge_runner.rs` — `merge_completed_sessions()`, `extract_completed_sessions()`, `MergeRunnerConfig`
- `crates/assay-core/src/pipeline.rs` — `build_harness_profile()`, `PipelineConfig`, `setup_session()`, `execute_session()`
- T01 output — MCP tool pattern confirms the session runner closure construction approach

## Expected Output

- `crates/assay-cli/src/commands/run.rs` — extended with multi-session routing, new flags, orchestration response types, and tests
