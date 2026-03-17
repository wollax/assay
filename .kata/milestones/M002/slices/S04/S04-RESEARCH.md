# S04: Exit Code 2 + Result Collection Compatibility — Research

**Date:** 2026-03-17

## Summary

S04 has two independent deliverables: (1) distinguish `assay run` exit code 2 (gate failures) from exit code 1 (pipeline error) and propagate it as exit code 2 from the `smelt run` process; (2) verify `ResultCollector::collect()` handles the post-Assay-merge git state correctly. Both are low-risk, narrowly-scoped changes.

**Exit code 2**: The fix is surgical — Phase 7's `exec_future` in `execute_run()` currently calls `anyhow::bail!` on any `handle.exit_code != 0`, which treats a `2` the same as a crash. The change: when `exit_code == 2`, print the distinct message, still proceed to result collection, and return `Ok(2)` from the closure instead of bailing. A new `JobPhase::GatesFailed` variant is added to `monitor.rs` so the monitor state file reflects the correct phase. The existing `ExecOutcome::Completed(Ok(code))` match arm already propagates the integer to `std::process::exit(c)` in `main.rs` — no change needed there.

**ResultCollector**: `collect()` reads `HEAD` and computes the delta against `base_ref`. Assay's merge phase advances `HEAD` on the bind-mounted repo (commits are already on the host filesystem per D013/D032). The existing `no_changes` guard handles the case where no sessions merged (HEAD == base_ref). All existing code paths are already compatible. S04 only needs a unit test to verify the invariant explicitly — no code changes to `collector.rs`.

## Recommendation

Two targeted changes to existing files:

1. **`crates/smelt-core/src/monitor.rs`**: Add `GatesFailed` variant to `JobPhase` enum.
2. **`crates/smelt-cli/src/commands/run.rs`**: In Phase 7 `exec_future`, add an `exit_code == 2` branch that (a) prints the distinct message, (b) skips the `bail!`, (c) captures the exit code to return after the collect block. In the `ExecOutcome` match, add an explicit `Ok(2)` arm that sets `GatesFailed` phase.
3. **`crates/smelt-core/src/collector.rs`**: Add a unit test (`test_collect_after_merge_commit`) that creates a repo, makes a merge commit (HEAD advances via `git merge --no-ff`), calls `collect()`, and asserts the branch is created with the correct commit count. This proves the post-Assay invariant without any code changes.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Propagating non-zero exit codes through the process | `main.rs` line 47: `std::process::exit(c)` where `c` is returned from `execute()` | Already the correct exit path; returning `Ok(2)` from `exec_future` propagates through `ExecOutcome::Completed(Ok(2))` unchanged |
| Gate failures — still collect results | Existing collect block already in `exec_future` after the `bail!` | Just move the `bail!` guard to only fire on `exit_code != 0 && exit_code != 2`; reuse collect block unchanged |
| ResultCollector post-merge state | `test_collect_basic` in `collector.rs` | Already covers HEAD-ahead-of-base; extend with a merge-commit variant for explicit Assay proof |
| Monitor phase transitions | `monitor.set_phase(JobPhase::...)` pattern throughout `execute_run()` | Add `GatesFailed` alongside existing variants; same pattern |

## Existing Code and Patterns

- `crates/smelt-cli/src/commands/run.rs:233–249` — Phase 7 `exec_future`: contains the `exit_code != 0` bail; this is the only change point for exit code 2 handling
- `crates/smelt-cli/src/commands/run.rs:276–303` — `tokio::select!` outcome match: add `ExecOutcome::Completed(Ok(2)) => { set_phase(GatesFailed); Ok(2) }` before the generic `Ok(code)` arm
- `crates/smelt-core/src/monitor.rs:21–30` — `JobPhase` enum: append `GatesFailed` as new variant; serde renames to `gates_failed` automatically via `rename_all = "snake_case"`
- `crates/smelt-core/src/collector.rs:42–74` — `collect()` impl: only reads `HEAD` and `base_ref`; no Assay-specific assumptions; fully compatible with post-merge state
- `crates/smelt-core/src/collector.rs:80–240` — existing unit tests: `test_collect_basic`, `test_collect_no_changes`, `test_collect_target_already_exists`, `test_collect_multiple_commits` — confirm current behavior; S04 adds one merge-commit variant

## Constraints

- **`JobPhase` is serde-serialized to TOML** (`run-state.toml`): adding `GatesFailed` is a non-breaking additive change; existing files without `gates_failed` will not parse to this variant; the monitor file is transient (cleaned up after each run) so no migration is needed
- **`exec_future` returns `Ok::<i32, anyhow::Error>`**: returning `Ok(2)` when `exit_code == 2` requires storing the exit code before the collect block runs, or restructuring — the cleanest approach is to save `let assay_exit = handle.exit_code;` before branching, then return `Ok(assay_exit)` at the end of the closure for both `0` and `2` cases
- **Collect runs on exit code 2**: This is intentional — Assay may have merged some session branches to the base before gate failures. Collecting captures partial results. The `no_changes` path in `ResultCollector` handles the zero-commits case gracefully.
- **D002 (firm)**: No `assay-types` dependency — no new imports needed; this slice only touches `monitor.rs`, `run.rs`, and `collector.rs` tests

## Common Pitfalls

- **Bail path bypasses collection**: The existing `if handle.exit_code != 0 { anyhow::bail!(...) }` guard comes before the collect block. If `exit_code == 2` takes this path, collection is skipped entirely. The fix must restructure the guard to only bail when `exit_code != 0 && exit_code != 2`.
- **Exit code 2 reaching the `Completed(Err(e))` arm**: If the `bail!` fires for exit_code=2 (before the fix), the `ExecOutcome::Completed(Err(e))` arm runs and sets `JobPhase::Failed`, returning `Err(e)` — which causes `main.rs` to print "Error: ..." and `process::exit(1)`. This is the current broken behavior to fix.
- **`GatesFailed` missing from monitor deserialize**: If `run-state.toml` contains `phase = "gates_failed"` and the binary was built without `GatesFailed`, deserialization fails. Since the state file is ephemeral (same binary version as the writer), this is not a real concern — but note it for the record.
- **`exec_future` return type**: The closure returns `Ok::<i32, anyhow::Error>(0)` at the end. If a merge commit scenario leaves `HEAD == base_ref` (e.g., Assay ran nothing), `collect()` returns `no_changes: true` — this is not an error, just `Ok(exit_code)`. No special handling needed.

## Open Risks

- **Partial collection after exit code 2**: When `assay run` exits 2 (some sessions failed gate checks), Assay may have merged zero, one, or several sessions to the base. `ResultCollector` is agnostic — it reads whatever HEAD is. If nothing merged, `no_changes: true`. This is correct behavior and requires no special casing.
- **Monitor state on gate failures**: The `JobPhase::GatesFailed` variant is new. `smelt status` will display it. If any existing code does an exhaustive match on `JobPhase`, adding the variant will cause a compile error — forcing explicit handling, which is the desired behavior.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust / Cargo | — | none found (standard) |
| tokio / async | — | none found (project-specific) |

## Sources

- `execute_run()` Phase 7 current implementation — shows bail-on-any-nonzero pattern and where `Ok(2)` path must be inserted (source: `crates/smelt-cli/src/commands/run.rs:233–303`)
- `JobPhase` enum — no `GatesFailed` variant yet (source: `crates/smelt-core/src/monitor.rs:21–30`)
- `main.rs` exit code propagation — `std::process::exit(c)` accepts the `i32` from `execute()` unchanged (source: `crates/smelt-cli/src/main.rs:47`)
- `ResultCollector::collect()` — reads HEAD, no Assay-specific assumptions, `no_changes` guard handles zero-delta case (source: `crates/smelt-core/src/collector.rs:42–74`)
- Assay exit code semantics: 0 = all merged, 1 = pipeline error, 2 = gate failures/merge conflicts (source: M002-RESEARCH.md § "Exit codes")
- S03 forward intelligence: `exec_streaming()` populates `ExecHandle.stderr` on all paths; Phase 7 callback is `|chunk| eprint!("{chunk}")`; these are unchanged by S04 (source: `.kata/milestones/M002/slices/S03/S03-SUMMARY.md`)
