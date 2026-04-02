---
estimated_steps: 4
estimated_files: 2
---

# T03: Implement exec() with streaming output and exit code

**Slice:** S02 — Docker Container Provisioning & Teardown
**Milestone:** M001

## Description

Implements `exec()` — the key risk being retired in S02. Creates an exec instance inside a running container, attaches to stdout/stderr streams, consumes the stream printing output in real time, and retrieves the exit code via `inspect_exec` after the stream completes. Tests exercise both successful commands, non-zero exit codes, and a multi-second command to validate stream reliability for longer-running processes.

## Steps

1. Implement `exec()` in `DockerProvider`:
   - Create exec instance via `create_exec` with `CreateExecOptions { cmd, attach_stdout: true, attach_stderr: true, .. }`.
   - Start exec via `start_exec` with `StartExecOptions { detach: false, .. }` (or default).
   - Pattern-match `StartExecResults::Attached { output, .. }` — this is the only valid variant for attached execution.
   - Consume the `output` stream using `StreamExt::next()` in a loop. For each `LogOutput::StdOut { message }` and `LogOutput::StdErr { message }`, print to stderr (or use `tracing::info!`). Collect stdout and stderr into separate `String` buffers for `ExecHandle` / later use.
   - After stream exhaustion, call `inspect_exec` to get `ExecInspectResponse`. Extract `exit_code` (it's `Option<i64>` — default to -1 if None).
   - Return `ExecHandle { container, exec_id }`.
   - Note: The current `ExecHandle` doesn't carry exit code or output. If the trait doesn't support returning these, store them on the `DockerProvider` (e.g., in a `HashMap<String, ExecResult>`) or expand `ExecHandle` fields. Alternatively, print output inline and let the caller infer success from the exec handle — check what the trait actually requires and adapt.
2. Add `tracing::info!` events: exec create, exec start, exec complete (with exit code). Log output lines at `tracing::debug!` level to avoid noise in normal operation.
3. Update integration tests in `docker_lifecycle.rs`:
   - `test_exec`: provision alpine:3, exec `["echo", "hello world"]`, verify stdout contains "hello world", exit code 0, teardown.
   - `test_exec_nonzero_exit`: provision, exec `["sh", "-c", "exit 42"]`, verify exit code 42, teardown.
   - Add `test_exec_long_running`: provision, exec `["sh", "-c", "for i in 1 2 3; do echo step-$i; sleep 1; done"]`, verify all 3 steps appear in output and exit code 0 — this retires the bollard exec stream reliability risk.
4. Verify all exec-related integration tests pass. Confirm no containers leaked.

## Must-Haves

- [ ] `exec()` creates exec instance, attaches to output stream, and consumes it fully
- [ ] Exit code retrieved via `inspect_exec` after stream completion (not from stream itself)
- [ ] Non-zero exit codes correctly reported (not swallowed or mapped to errors)
- [ ] Output streamed in real time (not buffered until completion)
- [ ] Multi-second command completes with all output captured (bollard exec reliability retired)

## Verification

- `cargo test -p smelt-cli -- docker_lifecycle::test_exec` passes
- `cargo test -p smelt-cli -- docker_lifecycle::test_exec_nonzero_exit` passes
- `cargo test -p smelt-cli -- docker_lifecycle::test_exec_long_running` passes
- `docker ps -a --filter label=smelt.job` returns empty after tests

## Observability Impact

- Signals added/changed: `tracing::info` events for exec create, exec start, exec complete with exit code. `tracing::debug` for individual output lines. `SmeltError::Provider` for exec failures with operation "exec".
- How a future agent inspects this: Exec output is streamed to stderr/tracing in real time. Exit codes are returned in `ExecHandle` or accessible via provider state.
- Failure state exposed: Bollard exec attach failures, stream errors, and missing exit codes all produce `SmeltError::Provider` with descriptive context.

## Inputs

- `crates/smelt-core/src/docker.rs` — `DockerProvider` with working `provision()` and `teardown()` from T02
- `crates/smelt-core/src/provider.rs` — `ExecHandle` struct shape (may need adaptation)
- S02-RESEARCH.md — `StartExecResults::Attached` pattern, `inspect_exec` for exit code, stream consumption patterns

## Expected Output

- `crates/smelt-core/src/docker.rs` — working `exec()` implementation with streaming output and exit code retrieval
- `crates/smelt-cli/tests/docker_lifecycle.rs` — all exec integration tests passing
