---
id: S04
parent: M002
milestone: M002
provides:
  - JobPhase::GatesFailed variant in monitor.rs (serde: "gates_failed")
  - Exit-code-2 pass-through path in Phase 7 exec_future — assay run exit 2 bypasses bail, runs collect, returns Ok(2)
  - Distinct stderr message "Assay complete — gate failures (exit 2)" vs generic exit message
  - Ok(assay_exit) propagation from exec_future → std::process::exit(2) in main.rs
  - test_collect_after_merge_commit — verifies collect() handles --no-ff merge commits correctly
  - test_job_phase_gates_failed_serde — verifies GatesFailed round-trips through TOML serde
requires:
  - slice: S03
    provides: exec_streaming() and Phase 7 wired for assay run with streaming output
affects: []
key_files:
  - crates/smelt-core/src/monitor.rs
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-core/src/collector.rs
key_decisions:
  - D050 — Exit-code-2 path: save assay_exit binding before branching; return Ok(assay_exit) at closure end
  - D051 — ResultCollector merge-commit compatibility verified by unit test; no code changes to collector.rs required
patterns_established:
  - Distinct JobPhase variants for distinct semantic outcomes (Complete vs GatesFailed vs Failed)
  - Local Wrapper struct in serde tests when subject is a bare enum value and toml is the serialization target
observability_surfaces:
  - .smelt/run-state.toml writes phase = "gates_failed" when assay exits 2
  - stderr message "Assay complete — gate failures (exit 2)" distinguishes from "Assay complete — exit code: N"
  - smelt exits with code 2 (propagated via Ok(2) → result → std::process::exit(2) in main.rs)
drill_down_paths:
  - .kata/milestones/M002/slices/S04/tasks/T01-SUMMARY.md
  - .kata/milestones/M002/slices/S04/tasks/T02-SUMMARY.md
duration: ~30 minutes
verification_result: passed
completed_at: 2026-03-17
---

# S04: Exit Code 2 + Result Collection Compatibility

**`smelt run` now exits 2 (not 1) when `assay run` exits 2, writes `phase = "gates_failed"` to run-state.toml, collects partial results, and `ResultCollector::collect()` is unit-tested against Assay's post-merge-commit state — bringing smelt-core to 112 passing tests with zero regressions.**

## What Happened

Two focused tasks delivered the complete slice with no deviations from the plan:

**T01** made four surgical edits to Phase 7 in `run.rs` and one enum addition to `monitor.rs`:
- Added `GatesFailed` to `JobPhase` after `Cancelled`; the existing `#[serde(rename_all = "snake_case")]` derive handles serialization automatically as `"gates_failed"`.
- Captured `handle.exit_code` into `let assay_exit` before any branching so both 0 and 2 propagate correctly via `Ok::<i32, anyhow::Error>(assay_exit)` at closure end.
- Split the eprintln! into a conditional: exit 2 prints `"Assay complete — gate failures (exit 2)"`, all other codes print the generic `"Assay complete — exit code: {assay_exit}"`.
- Changed the bail guard from `handle.exit_code != 0` to `assay_exit != 0 && assay_exit != 2` — exit 2 falls through to the collect block.
- Inserted `ExecOutcome::Completed(Ok(2)) => { monitor.set_phase(JobPhase::GatesFailed); Ok(2) }` as the first arm before the generic `Ok(code)` arm that sets `JobPhase::Complete`.

**T02** added two unit tests:
- `test_collect_after_merge_commit` in `collector.rs`: creates a feature branch with one commit, merges with `git merge --no-ff`, calls `collect()`, asserts `commit_count == 2`, `!no_changes`, non-empty `files_changed`, and branch HEAD matches merge commit. No code changes to `collector.rs` were needed — the implementation already handled merge commits correctly.
- `test_job_phase_gates_failed_serde` in `monitor.rs`: wraps `JobPhase::GatesFailed` in a local `Wrapper { phase: JobPhase }` struct (required because `toml` does not support top-level bare enum values), serializes to TOML, asserts output contains `"gates_failed"`, and asserts round-trip deserializes correctly.

## Verification

```
# Full workspace — zero regressions
cargo test --workspace
# smelt-core: test result: ok. 112 passed; 0 failed
# All other crates: ok, 0 failed

# Specific new tests
cargo test -p smelt-core test_collect_after_merge_commit -- --nocapture  → ok. 1 passed
cargo test -p smelt-core test_job_phase_gates_failed_serde -- --nocapture → ok. 1 passed

# Build clean — zero errors
cargo build --workspace 2>&1 | grep -c "^error"  → 0
```

## Deviations

**T02 serde test** — The task plan called for `toml::to_string(&JobPhase::GatesFailed)` directly, but `toml` returns `Err(UnsupportedType(Some("JobPhase")))` for bare enum values. Fixed by wrapping in a local `Wrapper { phase: JobPhase }` struct — accurately exercises the same serde path used in the real `RunState` struct. Test intent fully preserved.

No other deviations.

## Known Limitations

- The exit-code-2 path is only reachable with a real `assay` binary in the container — no integration test exercises it end-to-end (by slice-plan design; operational proof requires manual UAT with real Claude API key).
- `ResultCollector` merge-commit compatibility is verified at unit level only; the actual Assay post-session merge commit structure could differ if Assay changes its branching strategy.

## Follow-ups

- Manual UAT: `smelt run` with real Docker, real `assay` binary, and real Claude API key — demonstrates complete M002 pipeline producing a result branch (final operational proof).
- M002 milestone is now complete; all success criteria are covered by S01–S04.

## Files Created/Modified

- `crates/smelt-core/src/monitor.rs` — Added `GatesFailed` variant to `JobPhase` enum; added `test_job_phase_gates_failed_serde` unit test
- `crates/smelt-cli/src/commands/run.rs` — Phase 7: `assay_exit` binding, conditional message, updated bail guard, `Ok(assay_exit)` return, new `Ok(2)` outcome arm
- `crates/smelt-core/src/collector.rs` — Added `test_collect_after_merge_commit` unit test

## Forward Intelligence

### What the next slice should know
- M002 is complete — all success criteria from the roadmap are met. The remaining gap is manual UAT requiring a real Claude API key.
- The `GatesFailed` phase is now in `RunState` — any future `smelt status` display work should render it distinctly from `Failed` and `Complete`.

### What's fragile
- The `Ok(2)` arm in the outcome match must remain before the generic `Ok(code)` arm — Rust pattern matching is order-sensitive here; swapping them would route exit-2 to `JobPhase::Complete` silently.
- `toml::to_string` on bare enum values fails silently with `UnsupportedType` — always wrap in a struct for serde round-trip tests.

### Authoritative diagnostics
- `cat .smelt/run-state.toml` after a gate-failure run — look for `phase = "gates_failed"`; this is the ground-truth signal that the exit-2 path fired correctly.
- `echo $?` after `smelt run` on gate failure → must be `2`, not `1`.

### What assumptions changed
- Original assumption: `collect()` might need changes to handle merge commits. Actual: no code changes needed — `rev_list_count`/`diff_name_only` handle merge parents correctly by default. The test makes this invariant explicit.
