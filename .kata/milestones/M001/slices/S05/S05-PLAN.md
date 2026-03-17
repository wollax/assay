# S05: Job Monitoring, Timeout & Graceful Shutdown

**Goal:** Running jobs can be monitored with `smelt status`, timed out automatically, and gracefully shut down with Ctrl+C — all with guaranteed container cleanup.
**Demo:** Start a long-running `smelt run`, observe `smelt status` output in another terminal, send Ctrl+C, confirm all containers are removed.

## Must-Haves

- `JobMonitor` struct in smelt-core that tracks job phase, container ID, session names, start time, PID, and writes state to `.smelt/run-state.toml`
- `smelt status` CLI subcommand that reads `.smelt/run-state.toml` and prints formatted live progress (job name, elapsed time, current phase, container)
- Timeout enforcement: the assay exec future is wrapped in `tokio::select!` with `tokio::time::sleep`, and timeout triggers teardown + cleanup
- Ctrl+C handling: `tokio::signal::ctrl_c()` in the same `tokio::select!`, triggers graceful teardown with no orphaned containers
- State file cleanup on normal exit and signal/timeout paths
- Double teardown is safe (already guaranteed by DockerProvider's 404 tolerance)

## Proof Level

- This slice proves: operational
- Real runtime required: yes (Docker daemon for integration tests)
- Human/UAT required: no (automated tests cover timeout and signal paths; manual Ctrl+C is an operational confirmation for S06)

## Verification

- `cargo test -p smelt-core -- monitor::tests` — unit tests for JobMonitor state transitions, TOML serialization, file write/read/cleanup
- `cargo test -p smelt-core -- timeout::tests` — unit tests for timeout computation logic
- `cargo test -p smelt-cli -- status` — unit/integration tests for the `smelt status` subcommand (reads state file, handles missing/stale file)
- `cargo test -p smelt-cli --test docker_lifecycle -- timeout` — integration test: exec with a short timeout triggers teardown
- `cargo test -p smelt-cli --test docker_lifecycle -- signal` — integration test: cancellation via `CancellationToken` triggers teardown
- `cargo test --workspace` — all 121+ tests still pass, zero regressions

## Observability / Diagnostics

- Runtime signals: `JobMonitor` writes phase transitions to `.smelt/run-state.toml` with timestamps — phases are `provisioning`, `writing_manifest`, `executing`, `collecting`, `tearing_down`, `complete`, `failed`, `timeout`, `cancelled`
- Inspection surfaces: `smelt status` reads and formats the state file; `cat .smelt/run-state.toml` for raw state
- Failure visibility: State file records `phase = "failed"` or `phase = "timeout"` with timestamp; PID recorded for stale-state detection
- Redaction constraints: No secrets in state file — only job name, container short ID, session names, phase, timestamps, PID

## Integration Closure

- Upstream surfaces consumed: `execute_run()` in `run.rs` (orchestration hub, D026 pattern), `DockerProvider::teardown()` (404-tolerant), `AssayInvoker::build_run_command()` (max timeout computation)
- New wiring introduced in this slice: `tokio::select!` wrapping the exec block in `execute_run()` with timeout + signal branches; `JobMonitor` state file lifecycle; `smelt status` CLI subcommand
- What remains before the milestone is truly usable end-to-end: S06 integration — full multi-session pipeline through real `smelt run` entrypoint with all subsystems composed

## Tasks

- [x] **T01: Add JobMonitor and timeout helpers to smelt-core** `est:30m`
  - Why: Core monitoring state and timeout logic must exist before the CLI can use them. This creates the `monitor.rs` and timeout computation module, with full unit tests.
  - Files: `crates/smelt-core/src/monitor.rs`, `crates/smelt-core/src/lib.rs`, `crates/smelt-core/Cargo.toml`
  - Do: Create `JobMonitor` struct that tracks job phase/container/sessions/start time/PID, serializes to TOML, writes/reads/cleans up `.smelt/run-state.toml`. Add `compute_job_timeout()` helper that extracts max session timeout from manifest (reusing the logic from `AssayInvoker::build_run_command()`). Write comprehensive unit tests.
  - Verify: `cargo test -p smelt-core -- monitor::tests` passes
  - Done when: `JobMonitor` can write, read, and clean up state files; timeout computation works correctly; all unit tests pass

- [x] **T02: Add `smelt status` CLI subcommand** `est:25m`
  - Why: Users need to check job progress from another terminal. This wires the `status` subcommand into the CLI.
  - Files: `crates/smelt-cli/src/commands/status.rs`, `crates/smelt-cli/src/commands/mod.rs`, `crates/smelt-cli/src/main.rs`
  - Do: Add `Status` variant to `Commands` enum with optional `--dir` arg for project root. Read `.smelt/run-state.toml`, format and print status (job name, elapsed time, phase, container, sessions). Handle missing file (exit 1 with "no running job") and stale PID detection. Write tests for formatting and edge cases.
  - Verify: `cargo test -p smelt-cli -- status` passes; `cargo run -- status` with no state file prints "no running job"
  - Done when: `smelt status` reads state file, prints formatted output, handles missing/stale states, returns exit code 1 when no job running

- [x] **T03: Wire timeout, signal handling, and monitor into execute_run()** `est:35m`
  - Why: This is the integration point — the existing `execute_run()` async block must be wrapped with `tokio::select!` for timeout + Ctrl+C, and `JobMonitor` must be updated at each phase transition. Integration tests verify the real Docker path.
  - Files: `crates/smelt-cli/src/commands/run.rs`, `crates/smelt-cli/tests/docker_lifecycle.rs`, `crates/smelt-cli/Cargo.toml`
  - Do: In `execute_run()`: (1) create `JobMonitor` and write initial state after manifest load, (2) wrap the exec phase in `tokio::select!` with three branches: exec completion, `tokio::time::sleep(timeout)`, `tokio::signal::ctrl_c()`, (3) update monitor phase at each transition (provisioning → writing_manifest → executing → collecting → complete/failed/timeout/cancelled), (4) ensure teardown runs in all paths, (5) clean up state file after teardown. Add integration tests: one that verifies timeout triggers teardown (short timeout, long sleep command), one that verifies cancellation via `tokio_util::sync::CancellationToken` (simulating Ctrl+C in tests).
  - Verify: `cargo test -p smelt-cli --test docker_lifecycle -- timeout` and `-- signal` pass; `cargo test --workspace` all green
  - Done when: `execute_run()` enforces timeouts, handles Ctrl+C, updates monitor state at each phase, cleans up state file; all workspace tests pass including new integration tests

## Files Likely Touched

- `crates/smelt-core/src/monitor.rs` (new)
- `crates/smelt-core/src/lib.rs`
- `crates/smelt-core/Cargo.toml`
- `crates/smelt-cli/src/commands/status.rs` (new)
- `crates/smelt-cli/src/commands/mod.rs`
- `crates/smelt-cli/src/commands/run.rs`
- `crates/smelt-cli/src/main.rs`
- `crates/smelt-cli/tests/docker_lifecycle.rs`
- `crates/smelt-cli/Cargo.toml`
