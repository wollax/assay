# S02: Atomic state file — write on every transition — UAT

**Milestone:** M007
**Written:** 2026-03-23

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S02 delivers serialization infrastructure (free functions + wired mutations), not a user-facing behavior change. All correctness is verifiable via unit tests (round-trip, error tolerance, wiring). No live daemon startup or Docker execution is required to prove this slice's contracts.

## Preconditions

- `smelt` repo cloned; `cargo` available
- On the S02 branch (or main after merge)
- No `smelt serve` running — this UAT is purely file-level inspection

## Smoke Test

```
cargo test -p smelt-cli -- queue
```
Expected: 11 tests pass, 0 failed.

## Test Cases

### 1. Round-trip: state file written and readable

1. Run: `cargo test -p smelt-cli -- test_queue_state_round_trip`
2. **Expected:** test passes; TOML file written in a TempDir contains `[[jobs]]` entries; all fields (`id`, `manifest_path`, `source`, `attempt`, `status`, `queued_at`, `started_at`) read back equal to what was written.

### 2. Missing file tolerance

1. Run: `cargo test -p smelt-cli -- test_read_queue_state_missing_file`
2. **Expected:** test passes; `read_queue_state` on a non-existent directory returns an empty vec without panicking.

### 3. Corrupt file tolerance

1. Run: `cargo test -p smelt-cli -- test_read_queue_state_corrupt_file`
2. **Expected:** test passes; `read_queue_state` on garbage TOML content returns an empty vec without panicking; a `warn!` would appear in logs.

### 4. Enqueue wires write correctly

1. Run: `cargo test -p smelt-cli -- test_server_state_writes_on_enqueue`
2. **Expected:** test passes; state file exists after `enqueue`; `read_queue_state` on the same dir returns 1 job with `status == Queued`.

### 5. Full unit test suite — no regressions

1. Run: `cargo test -p smelt-cli`
2. **Expected:** ≥50 tests pass, 0 failed; all pre-existing serve tests (`test_enqueue_and_dispatch`, `test_complete_failure_retries`, `test_queue_cancel_queued`, etc.) appear in output unchanged.

## Edge Cases

### Atomic write — no partial file visible

1. Run: `cargo test -p smelt-cli -- test_queue_state_round_trip`
2. After test completes, inspect TempDir (if you capture it): `.smelt-queue-state.toml.tmp` should not be present.
3. **Expected:** only `.smelt-queue-state.toml` exists; `.tmp` file is absent after successful atomic write.

### TOML field names match spec

1. After running the round-trip test with a TempDir you can inspect:
   ```
   cat /tmp/<tempdir>/.smelt-queue-state.toml
   ```
2. **Expected:** file contains `[[jobs]]` header; fields include `id`, `manifest_path`, `source`, `attempt`, `status`, `queued_at`, `started_at`.

## Failure Signals

- Any `FAILED` in `cargo test -p smelt-cli` output
- `cargo check -p smelt-cli` emits new warnings (pre-existing `assert_cmd::cargo_bin` deprecations are unrelated)
- `.smelt-queue-state.toml.tmp` present after a test run (indicates failed rename)
- State file missing after `test_server_state_writes_on_enqueue` completes (indicates write path not wired)

## Requirements Proved By This UAT

- R028 (partial) — This UAT proves the write half of persistent queue: atomic writes occur on every durable mutation (`enqueue`/`complete`/`cancel`) and the state file can be read back faithfully. The read-on-startup recovery loop (the other half of R028) is proved by S03.

## Not Proven By This UAT

- R028 (full) — End-to-end restart recovery (`ServerState::load_or_new()`, re-dispatch of non-terminal jobs, attempt count preservation across restarts) is NOT proven here. That is S03's scope.
- Live daemon behavior — `smelt serve` is not started in this UAT; the state file is only exercised through unit tests with TempDir. Live operational verification (kill daemon, inspect file, restart, observe re-dispatch) is deferred to S03-UAT.md.
- `complete` and `cancel` write paths — Only `enqueue` is covered by a dedicated wiring test. `complete` and `cancel` paths are covered by the existing `test_complete_failure_retries` and `test_queue_cancel_queued` tests which still pass, but no dedicated file-existence assertion is made for those paths in this UAT.

## Notes for Tester

- All test cases are automated; no Docker or network access required.
- The `assert_cmd::cargo_bin` deprecation warnings in `cargo check` output are pre-existing and unrelated to S02 changes — safe to ignore.
- S02 does not change any user-visible CLI behavior; `smelt serve` still starts without persistence until S03 is merged.
