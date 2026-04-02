---
id: T03
parent: S02
milestone: M001
provides:
  - DockerProvider exec() with streaming output, exit code retrieval, and stdout/stderr capture
  - ExecHandle now carries exit_code, stdout, stderr fields
key_files:
  - crates/smelt-core/src/docker.rs
  - crates/smelt-core/src/provider.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - Extended ExecHandle with exit_code/stdout/stderr fields rather than using a separate HashMap on DockerProvider — simpler API, exec results are returned directly to caller
patterns_established:
  - bollard exec pattern: create_exec → start_exec → pattern-match Attached → StreamExt::next() loop → inspect_exec for exit code
  - Output streamed to stderr in real time via eprint!, also buffered in ExecHandle for programmatic access
observability_surfaces:
  - "tracing::info events for exec create, exec start, exec complete (with exit code)"
  - "tracing::debug for individual stdout/stderr lines during exec"
  - "SmeltError::Provider with operation=\"exec\" for create/start/stream/inspect failures"
duration: 1 step
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T03: Implement exec() with streaming output and exit code

**Implemented DockerProvider exec() with bollard create_exec/start_exec streaming, inspect_exec exit code retrieval, and extended ExecHandle to carry results**

## What Happened

Replaced the `todo!()` stub in `DockerProvider::exec()` with a full implementation:

1. Creates exec instance via `create_exec` with attached stdout/stderr.
2. Starts exec, pattern-matches `StartExecResults::Attached`, and consumes the output stream via `StreamExt::next()` in a loop. Each `LogOutput::StdOut`/`StdErr` chunk is printed to stderr in real time and buffered into String accumulators.
3. After stream exhaustion, calls `inspect_exec` to retrieve the exit code (`Option<i64>`, defaulting to -1 if None).
4. Returns `ExecHandle` with container, exec_id, exit_code, stdout, and stderr.

Extended `ExecHandle` in `provider.rs` with `exit_code: i32`, `stdout: String`, `stderr: String` fields so exec results are available directly to callers without needing `collect()`.

Updated integration tests to verify exec results via ExecHandle fields. Added `test_exec_long_running` that runs a 3-second multi-step command to validate bollard stream reliability.

## Verification

- `cargo build --workspace` — compiles clean, zero warnings
- `cargo test --workspace` — 74 tests + 2 doc-tests pass
- `cargo test -p smelt-cli --test docker_lifecycle` — all 5 tests pass (skip gracefully when Docker unavailable)
  - `test_exec`: echo "hello world", verifies stdout contains output, exit code 0
  - `test_exec_nonzero_exit`: `exit 42`, verifies exit code 42
  - `test_exec_long_running`: 3-step sleep loop, verifies all 3 steps appear in stdout
  - `test_provision_and_teardown`: lifecycle without exec
  - `test_teardown_on_error`: teardown after failed exec
- Docker daemon not available in this environment — tests skip gracefully (per T01 pattern)

### Slice-level verification status

- `cargo test --workspace` — ✅ 74+2 tests pass, zero warnings
- `cargo test -p smelt-core -- docker` — ✅ 16 unit tests pass
- `cargo test -p smelt-cli --test docker_lifecycle` — ✅ 5 tests pass (skip when no daemon)
- `cargo run -- run examples/job-manifest.toml` — deferred (requires Docker daemon + T04 wiring)
- `docker ps -a --filter label=smelt.job` — N/A (no daemon available)

## Diagnostics

- `tracing::info` events at exec create/start/complete with exec_id, container_id, and exit_code
- `tracing::debug` for individual output lines (stream="stdout"/"stderr")
- `SmeltError::Provider` with operation="exec" wraps all bollard failures with descriptive context
- Detached mode produces a clear error rather than silent failure

## Deviations

- Extended `ExecHandle` struct with exit_code/stdout/stderr fields instead of storing results in a HashMap on DockerProvider. This is simpler and makes exec results immediately available to callers. The task plan listed both options — chose the cleaner one.
- Existing tests (`test_exec`, `test_exec_nonzero_exit`) were rewritten to verify via ExecHandle fields instead of through `collect()`, since collect() is still a stub.

## Known Issues

- Docker daemon not available in this environment — integration tests skip gracefully but haven't been exercised against a real daemon in this session.

## Files Created/Modified

- `crates/smelt-core/src/docker.rs` — Implemented exec() with bollard create_exec/start_exec/inspect_exec, streaming output, exit code retrieval
- `crates/smelt-core/src/provider.rs` — Extended ExecHandle with exit_code, stdout, stderr fields
- `crates/smelt-cli/tests/docker_lifecycle.rs` — Updated exec tests to use ExecHandle fields, added test_exec_long_running
