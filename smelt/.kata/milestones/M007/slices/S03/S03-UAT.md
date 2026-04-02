# S03: Load-on-startup + restart-recovery integration test — UAT

**Milestone:** M007
**Written:** 2026-03-23

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: The recovery cycle (enqueue → write state file → drop ServerState → reconstruct via load_or_new → dispatch) is fully proven by unit tests with TempDir. The `serve.rs` wiring change is verified by `cargo check` and `cargo test`. No live Docker execution is required to prove persistence semantics — the critical invariants (state file written, jobs remapped, attempts preserved) are checked by deterministic unit tests with no external dependencies.

## Preconditions

- Rust toolchain available (`cargo`)
- `smelt-cli` builds cleanly: `cargo check -p smelt-cli` exits 0

## Smoke Test

```bash
cargo test -p smelt-cli -- load_or_new
```
Expected: 2 tests pass (`test_load_or_new_restart_recovery`, `test_load_or_new_missing_file`), 0 failed.

## Test Cases

### 1. Full restart-recovery cycle

1. Run `cargo test -p smelt-cli -- test_load_or_new_restart_recovery`
2. **Expected:** Test passes. Confirms: 3 jobs written to state file (Queued/0, Running/2, Queued/1), reconstructed by `load_or_new`, all 3 emerge as `Queued` with attempts 0, 2, 1 unchanged.

### 2. Cold-start (no prior state file)

1. Run `cargo test -p smelt-cli -- test_load_or_new_missing_file`
2. **Expected:** Test passes. Confirms: fresh TempDir with no state file, `load_or_new` returns empty queue, `queue_dir` is `Some(...)`, `max_concurrent` matches.

### 3. Wiring confirmed in serve.rs

1. Run `grep "load_or_new" crates/smelt-cli/src/commands/serve.rs`
2. **Expected:** Prints the wiring line: `ServerState::load_or_new(config.queue_dir.clone(), config.max_concurrent)`

### 4. Full test suite (no regressions)

1. Run `cargo test -p smelt-cli`
2. **Expected:** 52 passed, 0 failed.

## Edge Cases

### Corrupt state file (parse failure)

1. Write garbage bytes to `queue_dir/.smelt-queue-state.toml`
2. Call `load_or_new` on that directory
3. **Expected:** `read_queue_state` emits `warn!` with path and parse error; `load_or_new` returns empty queue; daemon starts cleanly (non-fatal).

### State file from a different queue_dir

1. Restart `smelt serve` with a different `queue_dir` than the previous run
2. **Expected:** No state file found, daemon starts with empty queue — no panic, no error, no stale jobs from the old path.

## Failure Signals

- `test_load_or_new_restart_recovery` fails — remapping logic or attempt preservation broken
- `test_load_or_new_missing_file` fails — cold-start path panics or queue_dir is None
- `grep "load_or_new" crates/smelt-cli/src/commands/serve.rs` returns nothing — wiring was reverted
- `cargo test -p smelt-cli` shows failures — regression in existing queue tests
- `cargo check -p smelt-cli` shows warnings — new code has lint issues

## Requirements Proved By This UAT

- R028 (`Persistent queue across smelt serve restarts`) — UAT cases 1–4 collectively prove the full R028 contract: jobs queued at crash time are re-queued on restart (`test_load_or_new_restart_recovery`), attempt counts preserved (case 1), cold-start works cleanly (case 2), wiring is live in the real daemon (case 3), no regressions (case 4)

## Not Proven By This UAT

- Live end-to-end kill-and-restart with real Docker jobs dispatched — the unit tests simulate the state file cycle without running actual containers; manual verification would require running `smelt serve`, enqueuing real jobs, killing the process, restarting, and observing re-dispatch in the TUI or logs
- `smelt status <job>` correctness for re-dispatched jobs after restart — out of scope for M007 (status reads per-job RunState, not the queue state file)
- Behavior when `queue_dir` is on a different filesystem or NFS mount (atomic rename semantics differ) — deferred

## Notes for Tester

- All verification is automated. The smoke test and full suite are the canonical proof.
- For manual kill-and-restart verification: set `queue_dir` in `server.toml`, run `smelt serve`, submit jobs via `POST /api/v1/jobs`, kill the process with `kill -9`, restart, and grep `.smelt/serve.log` for `"load_or_new: loaded N jobs"` to confirm recovery.
- The state file is at `<queue_dir>/.smelt-queue-state.toml` — `cat` it before restart to see what will be recovered.
