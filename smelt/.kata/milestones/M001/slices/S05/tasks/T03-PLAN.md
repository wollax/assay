---
estimated_steps: 5
estimated_files: 4
---

# T03: Wire timeout, signal handling, and monitor into execute_run()

**Slice:** S05 — Job Monitoring, Timeout & Graceful Shutdown
**Milestone:** M001

## Description

The core integration task: wrap the exec phase in `execute_run()` with a `tokio::select!` that races exec completion against timeout expiry and Ctrl+C signal. Integrate `JobMonitor` to update phase at each lifecycle stage. Add integration tests that verify timeout triggers teardown and cancellation triggers teardown — both against real Docker.

For testing signal handling, use `tokio_util::sync::CancellationToken` as a test-friendly abstraction instead of real `ctrl_c()`. The `execute_run()` function accepts an optional cancellation future, defaulting to `tokio::signal::ctrl_c()` in production and a `CancellationToken` in tests.

## Steps

1. Add `tokio-util` dependency to workspace `Cargo.toml` and `crates/smelt-cli/Cargo.toml` (dev-dependency for tests; the run.rs code can use a simpler approach — accept an `Option<tokio::sync::oneshot::Receiver<()>>` for cancellation in tests, or restructure the select to be testable). Actually, simpler: extract the select-wrapped exec into a helper function that takes a cancel future as a parameter. In production, pass `tokio::signal::ctrl_c()`. In tests, pass a controllable future.

2. Refactor `execute_run()` in `crates/smelt-cli/src/commands/run.rs`:
   - After manifest load + validation, create `JobMonitor::new(job_name, session_names, state_dir)` where `state_dir` is the manifest's parent dir or cwd joined with `.smelt/`
   - Call `monitor.write()` to persist initial state
   - Before provision: `monitor.set_phase(Provisioning)` (already initial)
   - After provision: `monitor.set_container(container_id)`, `monitor.set_phase(WritingManifest)`
   - After write manifest: `monitor.set_phase(Executing)`
   - Wrap the exec call in `tokio::select!`:
     ```rust
     tokio::select! {
         result = exec_future => { /* handle normal completion */ }
         _ = tokio::time::sleep(timeout) => { /* timeout path */ }
         _ = tokio::signal::ctrl_c() => { /* signal path */ }
     }
     ```
   - Timeout branch: `monitor.set_phase(Timeout)`, log timeout message, fall through to teardown
   - Signal branch: `monitor.set_phase(Cancelled)`, log cancellation message, fall through to teardown
   - After result collection: `monitor.set_phase(Collecting)`
   - After successful completion: `monitor.set_phase(Complete)`
   - On error: `monitor.set_phase(Failed)`
   - Teardown section: `monitor.set_phase(TearingDown)` before teardown call
   - After teardown: `monitor.cleanup()` to remove state file
   - Compute timeout via `compute_job_timeout(&manifest, SmeltConfig::default().default_timeout)`

3. Make the signal handling testable: Extract a `run_with_cancellation()` function that takes a cancellation future as a generic parameter. The public `execute_run()` passes `tokio::signal::ctrl_c()`. Tests pass a `tokio::sync::oneshot::Receiver` or similar controllable future.

4. Add integration tests in `crates/smelt-cli/tests/docker_lifecycle.rs`:
   - `test_timeout_triggers_teardown` — Create a manifest with a 1-session job, set timeout to 2 seconds, exec a `sleep 60` command. Verify: the function returns within ~3s, the container is removed, the error/exit indicates timeout.
   - `test_cancellation_triggers_teardown` — Use the testable cancellation interface: start a long exec, trigger cancellation after 1s, verify container is cleaned up. This tests the `select!` cancel branch without needing real SIGINT.

5. Verify all tests pass:
   - `cargo test -p smelt-cli --test docker_lifecycle -- timeout` 
   - `cargo test -p smelt-cli --test docker_lifecycle -- signal` or `cancel`
   - `cargo test --workspace` — all green, no regressions

## Must-Haves

- [ ] `execute_run()` wraps exec in `tokio::select!` with timeout + ctrl_c branches
- [ ] Timeout triggers teardown and returns appropriate error/exit code
- [ ] Ctrl+C triggers teardown and returns appropriate error/exit code
- [ ] `JobMonitor` updated at each phase transition throughout execute_run()
- [ ] State file cleaned up after teardown in all paths (success, error, timeout, cancel)
- [ ] Integration test proves timeout triggers container teardown against real Docker
- [ ] Integration test proves cancellation triggers container teardown against real Docker
- [ ] Double teardown remains safe (existing 404 tolerance in DockerProvider)

## Verification

- `cargo test -p smelt-cli --test docker_lifecycle -- timeout` — passes
- `cargo test -p smelt-cli --test docker_lifecycle -- cancel` — passes  
- `cargo test --workspace` — all tests pass, zero regressions
- Manual: `SMELT_LOG=info cargo run -- run examples/job-manifest.toml` shows phase transition messages on stderr

## Observability Impact

- Signals added/changed: Phase transition messages on stderr (`Provisioning...`, `Executing...`, `Timeout — tearing down...`, `Cancelled — tearing down...`); state file updated at each transition
- How a future agent inspects this: `cat .smelt/run-state.toml` during execution shows current phase; `smelt status` (from T02) reads the same file
- Failure state exposed: Timeout and cancellation produce distinct `phase` values (`timeout`, `cancelled`) in the state file and distinct error messages on stderr

## Inputs

- `crates/smelt-core/src/monitor.rs` — `JobMonitor`, `JobPhase`, `compute_job_timeout()` (from T01)
- `crates/smelt-cli/src/commands/run.rs` — existing `execute_run()` with async-block + teardown pattern (D026)
- `crates/smelt-cli/tests/docker_lifecycle.rs` — existing test helpers: `docker_provider_or_skip()`, `test_manifest_with_repo()`
- `crates/smelt-core/src/docker.rs` — `DockerProvider::teardown()` tolerates 404 (safe for double-teardown)

## Expected Output

- `crates/smelt-cli/src/commands/run.rs` — `execute_run()` refactored with `tokio::select!`, `JobMonitor` integration, timeout + signal handling
- `crates/smelt-cli/tests/docker_lifecycle.rs` — 2+ new integration tests (timeout, cancellation) that exercise real Docker teardown
- `crates/smelt-cli/Cargo.toml` — possible new dev-dependency if `tokio-util` needed for tests
