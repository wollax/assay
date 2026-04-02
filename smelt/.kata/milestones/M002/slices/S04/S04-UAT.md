# S04: Exit Code 2 + Result Collection Compatibility — UAT

**Milestone:** M002
**Written:** 2026-03-17

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S04 is contract + unit-level behavioral by design (slice plan: "Real runtime required: no; Human/UAT required: no"). The exit-code-2 path and merge-commit compatibility are verified entirely by unit tests and build checks. The operational proof (real Docker + real assay + real Claude key) is explicitly out of scope for this slice and deferred to manual UAT at milestone close.

## Preconditions

- Rust toolchain installed (`cargo` on PATH)
- No Docker daemon required
- Working directory: `/Users/wollax/Git/personal/smelt`

## Smoke Test

```bash
cargo build --workspace 2>&1 | grep -c "^error"
# Expected: 0
```

## Test Cases

### 1. Full workspace passes with zero regressions

```bash
cargo test --workspace 2>&1 | grep "test result"
```
**Expected:** All crates report `ok. N passed; 0 failed`. smelt-core shows 112 passed.

### 2. GatesFailed serializes to "gates_failed"

```bash
cargo test -p smelt-core test_job_phase_gates_failed_serde -- --nocapture
```
**Expected:** `test result: ok. 1 passed`

### 3. collect() handles merge commits correctly

```bash
cargo test -p smelt-core test_collect_after_merge_commit -- --nocapture
```
**Expected:** `test result: ok. 1 passed`; assertions: `commit_count == 2`, `!no_changes`, non-empty `files_changed`, branch HEAD matches merge commit.

### 4. GatesFailed variant present in source

```bash
grep -n "GatesFailed" crates/smelt-core/src/monitor.rs
```
**Expected:** Line showing `GatesFailed,` in the `JobPhase` enum.

### 5. Exit-code-2 bail guard present

```bash
grep -n "assay_exit != 2" crates/smelt-cli/src/commands/run.rs
```
**Expected:** Line showing `if assay_exit != 0 && assay_exit != 2`.

### 6. Ok(assay_exit) return propagates exit code

```bash
grep -n "Ok::<i32.*assay_exit" crates/smelt-cli/src/commands/run.rs
```
**Expected:** Line showing `Ok::<i32, anyhow::Error>(assay_exit)` at end of exec_future closure.

## Edge Cases

### GatesFailed must not match Complete arm in outcome match

```bash
grep -n "Ok(2)" crates/smelt-cli/src/commands/run.rs
```
**Expected:** An explicit `ExecOutcome::Completed(Ok(2))` arm appears before the generic `Ok(code)` arm — Rust exhaustive matching ensures correct routing.

### toml::to_string on bare enum returns error

This is documented in D051/T02 — the test wraps `JobPhase` in a `Wrapper` struct. The edge case is that naive `toml::to_string(&JobPhase::GatesFailed)` would silently fail (return `Err`) — the test's wrapper pattern prevents this regression.

## Failure Signals

- `cargo build --workspace` emitting any `^error` lines — indicates exhaustive match breakage or compile error
- `cargo test --workspace` showing any `FAILED` — regression in existing behavior
- `grep "GatesFailed" monitor.rs` returning nothing — variant was accidentally removed
- `grep "assay_exit != 2"` returning nothing — bail guard reverted to exit-code-0-only pass
- `test_job_phase_gates_failed_serde` failing with `UnsupportedType` — Wrapper struct removed

## Requirements Proved By This UAT

- No `.kata/REQUIREMENTS.md` exists — operating in legacy compatibility mode. S04 covers:
  - `assay run` exit code 2 is distinguished from exit code 1 — surfaces "gate failures" vs "pipeline error" (M002 success criterion 4)
  - `ResultCollector::collect()` handles Assay's merge-to-base-branch behavior correctly (M002 success criterion, verified by unit test)

## Not Proven By This UAT

- End-to-end `smelt run` with real Docker, real `assay` binary injected, and gate failures — requires live runtime
- `std::process::exit(2)` actually returns code 2 to the calling shell — requires a subprocess test or manual invocation
- `run-state.toml` written with `phase = "gates_failed"` in a real run — requires live runtime
- The stderr message "Assay complete — gate failures (exit 2)" appears on a real terminal — requires live runtime
- Assay's actual post-session merge commit structure matching the unit test's `--no-ff` simulation — requires real `assay` binary

## Notes for Tester

- S04 is intentionally unit-only. The operational proof (real `assay` + real Claude API key + end-to-end `smelt run`) is the manual UAT at M002 milestone closure.
- The `test_collect_after_merge_commit` test uses `git merge --no-ff` to replicate Assay's branching pattern. If Assay changes to `--squash` or fast-forward merges, this test's assumptions would need updating.
- `smelt-core` should show exactly 112 passing tests after S04. If the count differs, check for test-filter drift or accidental test removal.
