# S02: Docker Container Provisioning & Teardown — UAT

**Milestone:** M001
**Written:** 2026-03-17

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: The slice plan specifies "Human/UAT required: no". All verification is through automated tests (unit + integration) and CLI execution. The Docker lifecycle is exercised by integration tests against a real Docker daemon when available, and the CLI produces correct structured errors when the daemon is unavailable.

## Preconditions

- Rust toolchain installed (`cargo` available)
- Docker daemon running (for full lifecycle verification; tests skip gracefully without it)
- Repository cloned with S02 branch checked out

## Smoke Test

Run `cargo test --workspace` — all 96 tests pass with zero failures. Then run `cargo run -- run examples/job-manifest.toml` — if Docker is running, a container is provisioned, a health check executes with output streamed, and the container is removed. If Docker is not running, a clear connection error is reported.

## Test Cases

### 1. Resource string parsing

1. `cargo test -p smelt-core -- docker::tests`
2. **Expected:** 16 tests pass covering memory (G/M/K/bytes, fractional, edge cases) and CPU (integer, decimal, error cases) parsing

### 2. Container provision and teardown lifecycle

1. Start Docker daemon
2. `cargo test -p smelt-cli --test docker_lifecycle -- test_provision_and_teardown`
3. **Expected:** Container created from alpine:3 with smelt.job label, visible via inspect, then removed. Test passes.

### 3. Exec with streaming output

1. Start Docker daemon
2. `cargo test -p smelt-cli --test docker_lifecycle -- test_exec`
3. **Expected:** Command runs inside container, stdout captured in ExecHandle, exit code 0

### 4. Non-zero exit code handling

1. Start Docker daemon
2. `cargo test -p smelt-cli --test docker_lifecycle -- test_exec_nonzero_exit`
3. **Expected:** `exit 42` returns exit_code 42 in ExecHandle, no error thrown

### 5. Full CLI lifecycle

1. Start Docker daemon
2. `cargo run -- run examples/job-manifest.toml`
3. **Expected:** "Provisioning container..." → "Health check complete — exit code: 0" → "Container removed." printed to stderr. `docker ps -a --filter label=smelt.job` returns empty after.

### 6. Dry-run regression

1. `cargo run -- run examples/job-manifest.toml --dry-run`
2. **Expected:** Execution plan printed, no Docker interaction, exit 0

## Edge Cases

### Docker daemon unavailable

1. Stop Docker daemon (or run on machine without Docker)
2. `cargo run -- run examples/job-manifest.toml`
3. **Expected:** Clear error message mentioning Docker socket, exit 1. No crash or panic.

### Invalid manifest path

1. `cargo run -- run nonexistent.toml`
2. **Expected:** Error about file not found, exit 1

### Teardown after exec failure

1. Start Docker daemon
2. `cargo test -p smelt-cli --test docker_lifecycle -- test_teardown_on_error`
3. **Expected:** Container is removed even after exec failure. No orphaned containers.

## Failure Signals

- Any test failure in `cargo test --workspace`
- `docker ps -a --filter label=smelt.job` returning containers after a test or run completes
- CLI panicking instead of returning structured errors
- Missing lifecycle phase messages in stderr output during `smelt run`

## Requirements Proved By This UAT

No `.kata/REQUIREMENTS.md` exists. This UAT proves the S02 slice plan's must-haves: DockerProvider implements provision/exec/teardown, resource parsing works, containers are always cleaned up, and the CLI drives the full lifecycle.

## Not Proven By This UAT

- Repo bind-mount into container (S03)
- Assay execution inside container (S03)
- Result collection and branch extraction (S04)
- Timeout enforcement and Ctrl+C graceful shutdown (S05)
- Multi-session dependency ordering (S06)
- Real Docker daemon exercise in this specific session (daemon unavailable — tests skip gracefully)

## Notes for Tester

Docker daemon must be running for the lifecycle tests to actually exercise real containers. Without it, the tests skip gracefully and pass, but they don't prove the bollard integration. The `test_cli_run_lifecycle` test also checks for orphaned containers via `docker ps`. If running on a machine with other Smelt containers, filter results carefully.
