---
id: T01
parent: S04
milestone: M002
provides:
  - GatesFailed variant in JobPhase enum (monitor.rs)
  - Exit-code-2 pass-through path in Phase 7 exec_future (run.rs)
  - Distinct stderr message for gate failures
  - Ok(assay_exit) propagation from exec_future to std::process::exit
key_files:
  - crates/smelt-core/src/monitor.rs
  - crates/smelt-cli/src/commands/run.rs
key_decisions:
  - Inserted Ok(2) arm before generic Ok(code) arm — pattern-matching order in Rust ensures Ok(2) is caught first without disrupting complete/failed arms
patterns_established:
  - Distinct JobPhase variants for distinct semantic outcomes (Complete vs GatesFailed vs Failed) — callers can inspect run-state.toml to distinguish pipeline error from gate failure
observability_surfaces:
  - run-state.toml writes phase = "gates_failed" when assay exits 2
  - stderr message "Assay complete — gate failures (exit 2)" distinguishes from "Assay complete — exit code: N"
  - smelt exits with code 2 (propagated via Ok(2) → result → std::process::exit(2) in main.rs)
duration: ~10 minutes
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Add GatesFailed phase and wire exit-code-2 path in Phase 7

**Added `JobPhase::GatesFailed` variant and restructured Phase 7 so `assay run` exit code 2 bypasses the bail guard, runs the collect block, and returns `Ok(2)` → `std::process::exit(2)`.**

## What Happened

Two surgical edits:

1. **`monitor.rs`**: Appended `GatesFailed` as a new variant after `Cancelled` in the `JobPhase` enum. The existing `#[serde(rename_all = "snake_case")]` attribute on the enum automatically serializes it as `"gates_failed"` in `run-state.toml`.

2. **`run.rs`** — four changes in Phase 7's `exec_future` block and the outcome match:
   - Captured `handle.exit_code` into `let assay_exit` binding immediately after `exec_streaming` returns.
   - Replaced the single `eprintln!` with a conditional: exit 2 prints `"Assay complete — gate failures (exit 2)"`, all other codes print `"Assay complete — exit code: {assay_exit}"`.
   - Changed the bail guard from `handle.exit_code != 0` to `assay_exit != 0 && assay_exit != 2` — exit 2 no longer short-circuits before the collect block.
   - Changed the final return from `Ok::<i32, anyhow::Error>(0)` to `Ok::<i32, anyhow::Error>(assay_exit)` so both 0 and 2 propagate.
   - Inserted `ExecOutcome::Completed(Ok(2)) => { monitor.set_phase(JobPhase::GatesFailed); Ok(2) }` as the first arm in the outcome match, before the generic `Ok(code)` arm that sets `JobPhase::Complete`.

## Verification

```
# Clean build — zero errors
cargo build --workspace 2>&1 | grep "^error" | wc -l  → 0

# Variant present
grep -n "GatesFailed" crates/smelt-core/src/monitor.rs  → 28: GatesFailed,

# Bail guard updated
grep -n "assay_exit != 2" crates/smelt-cli/src/commands/run.rs  → 242: if assay_exit != 0 && assay_exit != 2 {

# Ok(assay_exit) present
grep -n "Ok::<i32.*assay_exit" crates/smelt-cli/src/commands/run.rs  → 276: Ok::<i32, anyhow::Error>(assay_exit)

# Full test suite
cargo test --workspace  → 155 tests (110 unit + 23 docker lifecycle + 10 dry_run + 10 cli + 2 doctest): all ok, 0 failed
```

## Diagnostics

- `cat .smelt/run-state.toml` after a gate-failure run → `phase = "gates_failed"`
- Stderr: `"Assay complete — gate failures (exit 2)"` is unique to exit-2 path
- `echo $?` after `smelt run` on gate failure → `2`

## Deviations

None — all steps executed exactly as written in the task plan.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/monitor.rs` — Added `GatesFailed` variant to `JobPhase` enum after `Cancelled`
- `crates/smelt-cli/src/commands/run.rs` — Phase 7: `assay_exit` binding, conditional message, updated bail guard, `Ok(assay_exit)` return, new `Ok(2)` outcome arm
