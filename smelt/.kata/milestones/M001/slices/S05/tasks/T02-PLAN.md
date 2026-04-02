---
estimated_steps: 4
estimated_files: 4
---

# T02: Add `smelt status` CLI subcommand

**Slice:** S05 — Job Monitoring, Timeout & Graceful Shutdown
**Milestone:** M001

## Description

Add the `smelt status` subcommand that reads the `.smelt/run-state.toml` state file written by `JobMonitor` and prints formatted job progress. Handles missing state files (no running job), stale PIDs (process died without cleanup), and formats elapsed time in human-readable form. Returns exit code 0 when a job is running, 1 when no job is found.

## Steps

1. Create `crates/smelt-cli/src/commands/status.rs`:
   - `StatusArgs` struct (clap `Args`): optional `--dir` path to project root (defaults to current directory)
   - `execute(args: &StatusArgs) -> Result<i32>` function:
     - Compute state dir as `{dir}/.smelt/`
     - Call `JobMonitor::read(state_dir)` — if file missing, print "No running job." to stderr, return `Ok(1)`
     - Check if PID is alive (`std::process::Command::new("kill").args(["-0", &pid.to_string()])` on Unix, or just skip on non-Unix) — if PID dead, print stale warning
     - Format and print: job name, phase, container ID (if set), sessions list, started at, elapsed time (compute from `started_at`)
     - Return `Ok(0)` for active job, `Ok(1)` for no job / stale

2. Register in `crates/smelt-cli/src/commands/mod.rs`: add `pub mod status;`

3. Wire into `crates/smelt-cli/src/main.rs`:
   - Add `Status(commands::status::StatusArgs)` variant to `Commands` enum with doc comment `/// Show status of a running job`
   - Add match arm in `main()`: `Commands::Status(ref args) => commands::status::execute(args).await`

4. Write tests in `status.rs` `#[cfg(test)] mod tests`:
   - `test_status_no_state_file` — execute against empty tempdir, verify exit code 1
   - `test_status_with_active_job` — write a RunState TOML file manually, execute, verify exit code 0 and output contains job name/phase
   - `test_status_stale_pid` — write state file with PID 99999999 (unlikely to exist), verify stale warning appears
   - Verify: `cargo test -p smelt-cli -- status` passes

## Must-Haves

- [ ] `smelt status` subcommand registered in CLI
- [ ] Reads `.smelt/run-state.toml` and prints formatted status
- [ ] Exit code 0 for active job, 1 for no job
- [ ] Handles missing state file gracefully with "No running job" message
- [ ] PID-based stale detection with warning
- [ ] 3+ unit tests for status command

## Verification

- `cargo test -p smelt-cli -- status` — all tests pass
- `cargo run -- status` with no state file prints "No running job." and exits 1
- `cargo test --workspace` — no regressions

## Observability Impact

- Signals added/changed: `smelt status` is itself an observability surface — it's the user-facing way to inspect a running job's state
- How a future agent inspects this: Run `smelt status` or `smelt status --dir /path/to/project`
- Failure state exposed: Stale PID detection warns when the runner process has died without cleanup

## Inputs

- `crates/smelt-core/src/monitor.rs` — `JobMonitor::read()`, `RunState`, `JobPhase` (from T01)
- `crates/smelt-cli/src/main.rs` — existing `Commands` enum and `main()` dispatch pattern

## Expected Output

- `crates/smelt-cli/src/commands/status.rs` — new module with `StatusArgs`, `execute()`, 3+ tests
- `crates/smelt-cli/src/commands/mod.rs` — updated with `pub mod status`
- `crates/smelt-cli/src/main.rs` — `Status` variant added to `Commands`, dispatched in `main()`
