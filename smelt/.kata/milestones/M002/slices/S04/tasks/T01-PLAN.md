---
estimated_steps: 7
estimated_files: 2
---

# T01: Add GatesFailed phase and wire exit-code-2 path in Phase 7

**Slice:** S04 — Exit Code 2 + Result Collection Compatibility
**Milestone:** M002

## Description

Two surgical edits: (1) append `GatesFailed` to the `JobPhase` enum in `monitor.rs` so the state file reflects gate failures as a distinct outcome; (2) restructure Phase 7 of `execute_run()` in `run.rs` so `assay run` exit code 2 does not trigger the bail path — instead it prints a distinct message, runs the collect block unchanged, and returns `Ok(2)` which propagates through `ExecOutcome::Completed(Ok(2))` → `std::process::exit(2)`.

The existing `ExecOutcome::Completed(Ok(code))` arm in the `tokio::select!` match sets `JobPhase::Complete` for any `Ok(code)`. A new explicit arm for `Ok(2)` must be inserted before this generic arm to set `JobPhase::GatesFailed` instead.

## Steps

1. Open `crates/smelt-core/src/monitor.rs`. Locate the `JobPhase` enum (lines ~19–30). Append `GatesFailed` as a new variant after `Cancelled`. The `#[serde(rename_all = "snake_case")]` attribute on the enum handles `"gates_failed"` serialization automatically. No other changes to `monitor.rs` are needed.

2. Open `crates/smelt-cli/src/commands/run.rs`. Locate the `exec_future` async block in Phase 7 (approximately lines 233–285). Find the line immediately after `exec_streaming()` returns its `ExecHandle`.

3. After the `handle` binding, add: `let assay_exit = handle.exit_code;`

4. Replace the existing `eprintln!("Assay complete — exit code: {}", handle.exit_code);` with a conditional:
   ```rust
   if assay_exit == 2 {
       eprintln!("Assay complete — gate failures (exit 2)");
   } else {
       eprintln!("Assay complete — exit code: {assay_exit}");
   }
   ```

5. Change the bail guard from:
   ```rust
   if handle.exit_code != 0 {
       anyhow::bail!("assay run exited with code {} — stderr: {}", handle.exit_code, handle.stderr.trim());
   }
   ```
   to:
   ```rust
   if assay_exit != 0 && assay_exit != 2 {
       anyhow::bail!("assay run exited with code {assay_exit} — stderr: {}", handle.stderr.trim());
   }
   ```

6. Replace the final `Ok::<i32, anyhow::Error>(0)` at the end of `exec_future` with `Ok::<i32, anyhow::Error>(assay_exit)`.

7. In the `tokio::select!` outcome match (approximately lines 288–310), insert a new arm before `ExecOutcome::Completed(Ok(code)) => { ... }`:
   ```rust
   ExecOutcome::Completed(Ok(2)) => {
       let _ = monitor.set_phase(JobPhase::GatesFailed);
       Ok(2)
   }
   ```
   The existing generic `Ok(code)` arm continues to set `JobPhase::Complete` for `Ok(0)` — no change needed there.

## Must-Haves

- [ ] `JobPhase::GatesFailed` variant exists in `monitor.rs` and is tagged with `serde(rename_all = "snake_case")` (inherited from enum attr)
- [ ] `exec_future` does not bail when `assay_exit == 2` — the collect block runs
- [ ] `exec_future` returns `Ok(assay_exit)` — both 0 and 2 are returned (not hardcoded `Ok(0)`)
- [ ] `ExecOutcome::Completed(Ok(2))` arm sets `JobPhase::GatesFailed` before the generic `Ok(code)` arm
- [ ] `cargo build --workspace` compiles clean — no exhaustive match breakage on `JobPhase`
- [ ] `cargo test --workspace` shows no FAILED (no regressions)

## Verification

```bash
# Must compile clean
cargo build --workspace 2>&1 | grep "^error" | wc -l
# Expected: 0

# Full test suite — no regressions
cargo test --workspace
# Expected: all "test result: ok."

# Confirm GatesFailed variant is present
grep -n "GatesFailed" crates/smelt-core/src/monitor.rs
# Expected: one match showing the variant

# Confirm bail guard updated
grep -n "assay_exit != 2" crates/smelt-cli/src/commands/run.rs
# Expected: one match at the guard

# Confirm Ok(assay_exit) at end of exec_future
grep -n "Ok::<i32.*assay_exit" crates/smelt-cli/src/commands/run.rs
# Expected: one match
```

## Observability Impact

- Signals added/changed: `run-state.toml` will contain `phase = "gates_failed"` when assay exits 2; previously this path hit `JobPhase::Failed` via the bail → `Completed(Err)` arm
- How a future agent inspects this: `cat .smelt/run-state.toml` during or after a gate-failure run; stderr message `"Assay complete — gate failures (exit 2)"` is distinct from `"Assay complete — exit code: N"` for other codes
- Failure state exposed: `JobPhase::GatesFailed` is written before teardown so the state file survives teardown and reflects the outcome; exit code 2 from `smelt run` distinguishes gate failures from pipeline errors in CI scripts

## Inputs

- `crates/smelt-core/src/monitor.rs` — `JobPhase` enum to extend
- `crates/smelt-cli/src/commands/run.rs` — Phase 7 `exec_future` block with current `handle.exit_code != 0` bail logic and `tokio::select!` outcome match; S03 left the collect block intact after the bail, so restructuring is minimal

## Expected Output

- `crates/smelt-core/src/monitor.rs` — `GatesFailed` variant appended to `JobPhase` enum
- `crates/smelt-cli/src/commands/run.rs` — Phase 7 uses `assay_exit` binding; bail guard excludes 2; distinct message for exit 2; `Ok(assay_exit)` at closure end; new `Ok(2)` arm in outcome match setting `GatesFailed`
