# S05: Job Monitoring, Timeout & Graceful Shutdown — UAT

**Milestone:** M001
**Written:** 2026-03-17

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: S05 is operational verification — timeout enforcement and signal handling require a real Docker daemon to prove teardown actually happens. Automated integration tests cover the critical paths (timeout → teardown, cancellation → teardown). Manual Ctrl+C confirmation is deferred to S06 full-pipeline UAT per the slice plan.

## Preconditions

- Docker daemon running (`docker info` succeeds)
- `cargo build` succeeds
- No stale smelt containers from prior test runs (`docker ps --filter label=smelt.job` should be empty)
- At `.smelt/run-state.toml` does not exist (or is stale from a prior run)

## Smoke Test

Run the status unit tests and one Docker integration test:
```
cargo test -p smelt-cli --lib -- status
cargo test -p smelt-cli --test docker_lifecycle -- timeout
```
Both should pass. If the timeout test passes, the full select!/teardown path is confirmed against real Docker.

## Test Cases

### 1. smelt status with no running job

```sh
# Ensure no state file exists
rm -f .smelt/run-state.toml
cargo run -- status
```
**Expected:** Prints "No running job." to stderr, exits with code 1.

### 2. smelt status with an active job

```rust
// Write a synthetic state file (as done in unit tests)
// Phase = "executing", PID = current process PID
```
```sh
cargo test -p smelt-cli --lib -- test_status_with_active_job
```
**Expected:** 7 status unit tests all pass, including active job formatting and stale PID detection.

### 3. Timeout triggers container teardown (automated Docker test)

```sh
cargo test -p smelt-cli --test docker_lifecycle -- test_timeout_triggers_teardown
```
**Expected:** Test passes in ~12s. Internally: container is provisioned, a `sleep 60` exec is started with a 2s timeout, timeout fires, teardown runs, container is removed. Test asserts no smelt-labeled container remains.

### 4. Cancellation triggers container teardown (automated Docker test)

```sh
cargo test -p smelt-cli --test docker_lifecycle -- test_cancellation_triggers_teardown
```
**Expected:** Test passes in ~12s. Internally: cancellation signal (oneshot channel) fires after exec starts, teardown runs, container is removed. Proves Ctrl+C path cleans up containers.

### 5. Double teardown is safe

```sh
cargo test -p smelt-cli --test docker_lifecycle -- test_double_teardown_safe
```
**Expected:** Test passes. DockerProvider::teardown() called twice on the same container does not panic or return an error.

## Edge Cases

### Missing state file

```sh
rm -f .smelt/run-state.toml
cargo run -- status
```
**Expected:** Exit code 1, "No running job." on stderr. No panic.

### State file with terminal phase (complete/failed/timeout/cancelled)

```sh
# Write a state file with phase = "complete" (as done in unit tests)
cargo test -p smelt-cli --lib -- test_is_terminal_phase
```
**Expected:** `is_terminal_phase()` returns true for all 4 terminal phases; `smelt status` would exit 1 for these.

### Stale PID (process died without cleanup)

```sh
cargo test -p smelt-cli --lib -- test_status_stale_pid
```
**Expected:** Status command prints stale PID warning to stderr, exits 1.

## Failure Signals

- `smelt status` exits 0 when it should exit 1 → terminal phase not being detected
- `test_timeout_triggers_teardown` leaves a container behind → select! or teardown broken
- `test_cancellation_triggers_teardown` hangs → cancellation future not being awaited
- `test_double_teardown_safe` panics → 404 tolerance not applied to remove_container
- `.smelt/run-state.toml` not cleaned up after test → state file lifecycle broken in teardown path

## Requirements Proved By This UAT

> **Note:** No `.kata/REQUIREMENTS.md` exists. Operating in legacy compatibility mode. Requirement IDs are described by capability.

- **Job monitoring (smelt status)** — Proved by status unit tests: reads state file, formats output, handles missing/stale states, returns correct exit codes.
- **Timeout enforcement** — Proved by `test_timeout_triggers_teardown`: short timeout fires before long-running exec completes, teardown removes container, no orphans.
- **Graceful cancellation (Ctrl+C path)** — Proved by `test_cancellation_triggers_teardown`: cancellation signal triggers teardown, container removed, no orphans.
- **Idempotent teardown** — Proved by `test_double_teardown_safe`: double teardown is safe, 404 tolerated on remove.
- **State file lifecycle** — Proved by monitor unit tests: written on start, updated at each phase, cleaned up after teardown.

## Not Proven By This UAT

- **Manual Ctrl+C via real terminal** — Deferred to S06 full-pipeline UAT. The automated cancellation test proves the same code path with a synthetic signal.
- **`smelt status` in a separate terminal while `smelt run` is active** — Requires a real long-running job. Confirmed by the state file lifecycle tests and status unit tests; full operational confirmation deferred to S06.
- **Multi-session job monitoring** — S05 establishes the single-job monitoring framework. Multi-session session-boundary updates are S06 scope.
- **Non-Unix PID liveness** — Windows/non-POSIX platforms skip the `kill -0` check. Untested.
- **State file persistence across process restart** — State file persists on disk between runs (only cleaned up by the runner itself). A new `smelt run` would overwrite it. Concurrent job safety not proved (single-job model per D034).

## Notes for Tester

- The 2 pre-existing Docker integration test failures (`test_collect_creates_target_branch`, `test_cli_run_lifecycle`) are not S05 regressions. `test_cli_run_lifecycle` leaves stale containers behind — run `docker rm -f $(docker ps -aq --filter label=smelt.job)` to clean up before running S05 Docker tests in isolation.
- Docker tests require `alpine:3` to be pullable. Tests skip gracefully if Docker daemon is unavailable (D024 pattern).
- The timeout integration test sleeps 12s (10s setup + 2s timeout fire) — this is expected and intentional.
