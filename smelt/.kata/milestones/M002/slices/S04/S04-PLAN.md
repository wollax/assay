# S04: Exit Code 2 + Result Collection Compatibility

**Goal:** `smelt run` exits with code 2 (not 1) when `assay run` exits 2 (gate failures), surfaces a distinct message, and still collects partial results; `ResultCollector::collect()` is verified by unit test to handle the post-Assay merge-commit state correctly.
**Demo:** `cargo test --workspace` passes with a new `test_collect_after_merge_commit` unit test; Phase 7 audit shows `GatesFailed` written to run-state.toml on exit-code-2 and `Ok(2)` propagated to `std::process::exit(2)`.

## Must-Haves

- `JobPhase::GatesFailed` variant exists in `monitor.rs` (serde: `gates_failed`)
- Phase 7 in `execute_run()` does NOT bail when `handle.exit_code == 2`; instead prints distinct message, proceeds to the collect block, and returns `Ok(2)`
- Phase 7 `ExecOutcome::Completed(Ok(2))` arm sets `JobPhase::GatesFailed` (not `Complete`)
- Exit code 1 (or any other non-zero, non-2 code) still bails, sets `JobPhase::Failed`, exits 1
- `test_collect_after_merge_commit` unit test in `collector.rs` creates a merge commit, calls `collect()`, and asserts branch created with correct commit count and HEAD pointing at merge commit

## Proof Level

- This slice proves: contract + unit-level behavioral
- Real runtime required: no (unit tests only; no Docker integration test added)
- Human/UAT required: no

## Verification

```bash
# Full workspace — no regressions
cargo test --workspace

# New merge-commit unit test
cargo test -p smelt-core test_collect_after_merge_commit -- --nocapture

# GatesFailed variant compiles and serializes correctly
cargo test -p smelt-core test_job_phase_gates_failed_serde

# Exit-code-2 path compiles correctly (verify no exhaustive match breakage)
cargo build --workspace 2>&1 | grep -c "^error" || true
```

## Observability / Diagnostics

- Runtime signals: `run-state.toml` will contain `phase = "gates_failed"` after a gate-failure run; existing `JobPhase::Failed` remains for pipeline errors (exit code 1 or other non-zero)
- Inspection surfaces: `.smelt/run-state.toml` — readable by `smelt status`; stderr message `"Assay complete — gate failures (exit 2)"` distinguishes from `"Assay complete — exit code: 1"`
- Failure visibility: Phase set before teardown, so state file reflects the gate-failure outcome even if teardown fails; `Ok(2)` return propagates to `std::process::exit(2)` in `main.rs` unchanged
- Redaction constraints: none — no credentials in phase names or exit code messages

## Integration Closure

- Upstream surfaces consumed: `crates/smelt-cli/src/commands/run.rs` Phase 7 (from S03); `crates/smelt-core/src/monitor.rs` `JobPhase` enum; `crates/smelt-core/src/collector.rs` `collect()` impl
- New wiring introduced in this slice: `GatesFailed` arm in `tokio::select!` outcome match; exit-code-2 branch inserted between `exec_streaming` return and existing bail
- What remains before the milestone is truly usable end-to-end: nothing — all M002 success criteria are now covered by S01–S04; manual UAT with real Claude API key is the remaining operational proof

## Tasks

- [x] **T01: Add GatesFailed phase and wire exit-code-2 path in Phase 7** `est:30m`
  - Why: Core behavioral change — `assay run` exit code 2 must not trigger the bail path; must set `GatesFailed` phase and return `Ok(2)` so result collection runs and the process exits 2
  - Files: `crates/smelt-core/src/monitor.rs`, `crates/smelt-cli/src/commands/run.rs`
  - Do:
    1. In `monitor.rs`, append `GatesFailed` to the `JobPhase` enum after `Cancelled`; the `#[serde(rename_all = "snake_case")]` derive handles serialization automatically
    2. In `run.rs` Phase 7 `exec_future`, save `let assay_exit = handle.exit_code;` immediately after `exec_streaming` returns, before any branching
    3. Change the guard `if handle.exit_code != 0 { anyhow::bail!(...) }` to `if assay_exit != 0 && assay_exit != 2 { anyhow::bail!(...) }` — this is the only structural change to the guard
    4. Before that guard, add: `if assay_exit == 2 { eprintln!("Assay complete — gate failures (exit 2)"); }` replacing the existing generic "exit code" message for this case (keep the generic message for non-2 exits on the bail path)
    5. Replace the final `Ok::<i32, anyhow::Error>(0)` at the bottom of `exec_future` with `Ok::<i32, anyhow::Error>(assay_exit)` so both 0 and 2 are propagated correctly
    6. In the `tokio::select!` outcome match, add an explicit arm `ExecOutcome::Completed(Ok(2)) => { let _ = monitor.set_phase(JobPhase::GatesFailed); Ok(2) }` before the generic `ExecOutcome::Completed(Ok(code))` arm
    7. Verify no exhaustive match on `JobPhase` elsewhere in the codebase breaks: run `cargo build --workspace`
  - Verify: `cargo build --workspace` compiles clean; `cargo test --workspace` passes (no regressions from monitor/run changes)
  - Done when: `cargo build --workspace` exits 0; `cargo test --workspace` shows no FAILED

- [x] **T02: Add merge-commit and serde unit tests to collector.rs and monitor.rs** `est:20m`
  - Why: Closes the explicit verification gap — proves `ResultCollector::collect()` handles Assay's post-merge HEAD state correctly, and proves `GatesFailed` round-trips through serde without silent corruption
  - Files: `crates/smelt-core/src/collector.rs`, `crates/smelt-core/src/monitor.rs`
  - Do:
    1. In `collector.rs` tests, add `test_collect_after_merge_commit`: using the existing `setup_test_repo()` and `add_commit()` helpers, capture `base = head_hash(tmp.path())`; create and switch to a feature branch (`git checkout -b feat`); add one commit on feat; switch back to main (`git checkout main` or whatever the default branch is); perform `git merge --no-ff feat -m "merge feat"` via `std::process::Command`; then call `collector.collect(&base, "results/after-merge")`; assert `!result.no_changes`, `result.commit_count == 2` (the feat commit + the merge commit), branch exists, branch HEAD matches `head_hash(tmp.path())`
    2. In `monitor.rs` tests (or inline if no test module exists yet), add `test_job_phase_gates_failed_serde`: serialize `JobPhase::GatesFailed` to TOML string and assert it contains `"gates_failed"`; deserialize `"gates_failed"` back and assert it equals `JobPhase::GatesFailed`
    3. Run both tests individually with `--nocapture` to confirm output
  - Verify: `cargo test -p smelt-core test_collect_after_merge_commit -- --nocapture` passes; `cargo test -p smelt-core test_job_phase_gates_failed_serde -- --nocapture` passes
  - Done when: Both new tests pass; `cargo test -p smelt-core` shows 112+ passed (was 110 before S04)

## Files Likely Touched

- `crates/smelt-core/src/monitor.rs`
- `crates/smelt-cli/src/commands/run.rs`
- `crates/smelt-core/src/collector.rs`
