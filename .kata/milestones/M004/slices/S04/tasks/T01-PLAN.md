---
estimated_steps: 6
estimated_files: 1
---

# T01: Rewrite `execute_mesh()` and `execute_gossip()` CLI Stubs with Real Session Runners

**Slice:** S04 — Integration + Observability
**Milestone:** M004

## Description

The current `execute_mesh()` and `execute_gossip()` functions in `run.rs` use `unreachable!()` as their session runner closure — any manifest that actually invokes them will panic. `run_mesh()` and `run_gossip()` in assay-core are fully implemented (S02/S03), so the only fix needed is wiring the real HarnessWriter session runner at the CLI call site (exactly what `execute_orchestrated()` does) and populating the `OrchestrationResponse` from outcomes instead of returning hardcoded zeros.

The template to follow is `execute_orchestrated()` lines ~354–585 in `run.rs`. Mesh/Gossip don't need the 3-phase flow (no checkout, no merge) — just Phase 1 (execute) and formatting.

## Steps

1. **Copy the HarnessWriter session runner from `execute_orchestrated()`** into `execute_mesh()`. Replace the current `unreachable!()` closure with the real one:
   ```rust
   let session_runner = |session: &assay_types::ManifestSession,
                         pipe_cfg: &assay_core::pipeline::PipelineConfig|
    -> Result<assay_core::pipeline::PipelineResult, assay_core::pipeline::PipelineError> {
       let harness_writer: Box<assay_core::pipeline::HarnessWriter> = Box::new(
           |profile: &assay_types::HarnessProfile, worktree_path: &std::path::Path| {
               let claude_config = assay_harness::claude::generate_config(profile);
               assay_harness::claude::write_config(&claude_config, worktree_path)
                   .map_err(|e| format!("Failed to write claude config: {e}"))?;
               Ok(assay_harness::claude::build_cli_args(&claude_config))
           },
       );
       assay_core::pipeline::run_session(session, pipe_cfg, &harness_writer)
   };
   ```

2. **Replace the leading eprintln in `execute_mesh()`** from `"Mesh mode manifest ({} session(s)) — routing to mesh executor stub"` to `"mode: mesh — {} session(s)"` (using `manifest.sessions.len()`).

3. **Populate `OrchestrationResponse` from `orch_result.outcomes`** in `execute_mesh()`. After calling `run_mesh()`, iterate outcomes:
   ```rust
   use assay_core::orchestrate::executor::SessionOutcome;
   let mut completed_count = 0usize;
   let mut failed_count = 0usize;
   let mut skipped_count = 0usize;
   let mut session_results = Vec::new();
   for (name, outcome) in &orch_result.outcomes {
       match outcome {
           SessionOutcome::Completed { .. } => { completed_count += 1; ... }
           SessionOutcome::Failed { error, .. } => { failed_count += 1; ... }
           SessionOutcome::Skipped { reason } => { skipped_count += 1; ... }
       }
   }
   ```
   Use the exact same eprintln pattern as `execute_orchestrated` (`[✓]`/`[✗]`/`[−]`) when `!cmd.json`. Populate `OrchestrationResponse { run_id, sessions: session_results, merge_report: empty_merge_report, summary: OrchestrationSummary { total, completed, failed, skipped, ... } }`.

4. **Repeat steps 2–3 for `execute_gossip()`**, changing the mode label to `"mode: gossip"`.

5. **Add two new unit tests** inside the `#[cfg(test)] mod tests` block:
   - `execute_mesh_output_mode_label`: create a `RunManifest` with `mode: OrchestratorMode::Mesh` and two sessions; assert `manifest.mode == OrchestratorMode::Mesh` and `manifest.sessions.len() == 2` (validates the manifest shape the function receives — does not actually call execute, just validates the routing precondition).
   - `execute_gossip_output_mode_label`: same for Gossip mode.
   These are lightweight guard tests that confirm the mode field is preserved through manifest construction and would catch accidental mode enum changes.

6. **Run `cargo clippy -p assay-cli --all-targets --features orchestrate -- -D warnings`** and fix any warnings introduced.

## Must-Haves

- [ ] `execute_mesh()` no longer contains `unreachable!()` — uses real HarnessWriter session runner
- [ ] `execute_gossip()` no longer contains `unreachable!()` — uses real HarnessWriter session runner
- [ ] Both functions print `"mode: mesh"` / `"mode: gossip"` to stderr on entry
- [ ] Both functions populate `OrchestrationResponse.sessions` from `orch_result.outcomes` (not hardcoded `vec![]`)
- [ ] Both functions populate `OrchestrationSummary` with real counts (not hardcoded zeros)
- [ ] `cargo clippy -p assay-cli --all-targets --features orchestrate -- -D warnings` exits 0
- [ ] All existing `cargo test -p assay-cli` tests pass

## Verification

- `cargo test -p assay-cli` — no test failures, no panics
- `cargo clippy -p assay-cli --all-targets --features orchestrate -- -D warnings` — exits 0
- `grep -c "unreachable" crates/assay-cli/src/commands/run.rs` — should be 0 in execute_mesh/execute_gossip function bodies (search specifically in those functions)

## Observability Impact

- Signals added/changed: CLI stderr now emits `"mode: mesh — N session(s)"` / `"mode: gossip — N session(s)"` for mesh/gossip runs; per-session `[✓]`/`[✗]`/`[−]` lines follow (matching DAG output format)
- How a future agent inspects this: `assay run manifest.toml 2>&1 | grep "mode:"` — confirms which mode was used; `assay run manifest.toml --json | jq '.sessions'` — non-empty array with per-session outcomes
- Failure state exposed: session failures now appear as `SessionOutcome::Failed` entries in the JSON response with `error.message` populated, instead of silent empty sessions list

## Inputs

- `crates/assay-cli/src/commands/run.rs:354–585` — `execute_orchestrated()` is the template; copy the HarnessWriter closure and outcome-iteration loop verbatim, drop the 3-phase checkout+merge logic
- `crates/assay-cli/src/commands/run.rs:587–740` — current `execute_mesh()` and `execute_gossip()` stubs to rewrite
- S02-SUMMARY.md and S03-SUMMARY.md confirm `run_mesh()` / `run_gossip()` accept the same `&SessionRunner` generic parameter as `run_orchestrated()`

## Expected Output

- `crates/assay-cli/src/commands/run.rs` — `execute_mesh()` and `execute_gossip()` rewritten: real HarnessWriter runner, mode label in stderr, outcomes from `orch_result.outcomes`, 2 new lightweight unit tests
