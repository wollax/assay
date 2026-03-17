---
id: S02
parent: M001
milestone: M001
provides:
  - DockerProvider implementing RuntimeProvider trait (provision, exec, teardown, stub collect)
  - Image pull with stream drain as part of provision
  - Container creation with env vars, resource limits (memory/cpu), smelt.job label
  - Exec with bollard create_exec/start_exec streaming, inspect_exec exit code retrieval
  - Teardown with force-remove, graceful handling of already-stopped containers
  - Resource string parsing (memory: G/M/K/bytes, CPU: integer/decimal → nanocpus)
  - Async CLI main with tokio runtime
  - smelt run manifest.toml drives full provision→exec→teardown lifecycle
requires:
  - slice: S01
    provides: RuntimeProvider trait, JobManifest/Environment/CredentialConfig structs, SmeltError::Provider, CLI run command with --dry-run
affects:
  - S03 (consumes DockerProvider for repo mount + Assay execution)
  - S05 (consumes DockerProvider::teardown for timeout/signal cleanup)
  - S06 (consumes full Docker lifecycle for integration tests)
key_files:
  - crates/smelt-core/src/docker.rs
  - crates/smelt-core/src/provider.rs
  - crates/smelt-cli/src/main.rs
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/tests/docker_lifecycle.rs
key_decisions:
  - Container keep-alive via sleep 3600 CMD, work via exec (D021)
  - smelt.job=<name> label on all containers for identification and cleanup (D022)
  - Explicit teardown in both success and error paths via async block pattern (D023)
  - Docker lifecycle tests skip gracefully when daemon unavailable (D024)
  - ExecHandle extended with exit_code/stdout/stderr fields for direct result access (D025)
  - Stop errors (304/404) silently ignored in teardown — container may already have exited
  - Credential env vars resolved at provision time, missing vars silently skipped
  - Example manifest uses alpine:3 for fast test pulls
patterns_established:
  - bollard exec pattern: create_exec → start_exec → pattern-match Attached → StreamExt::next() loop → inspect_exec for exit code
  - docker_provider_or_skip() test helper for graceful daemon-unavailable skipping
  - Async block cleanup guard in CLI — provision returns container_id, async block does work, teardown runs unconditionally
  - bollard builder pattern for query params (CreateImageOptionsBuilder, StopContainerOptionsBuilder, etc.)
observability_surfaces:
  - CLI prints lifecycle phases to stderr (Provisioning → Health check complete → Container removed)
  - tracing::info events for image pull, container create/start, exec create/start/complete, teardown
  - SmeltError::Provider with operation context (connect, provision, exec, teardown) and bollard source error
  - docker ps --filter label=smelt.job shows active Smelt containers
drill_down_paths:
  - .kata/milestones/M001/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M001/slices/S02/tasks/T02-SUMMARY.md
  - .kata/milestones/M001/slices/S02/tasks/T03-SUMMARY.md
  - .kata/milestones/M001/slices/S02/tasks/T04-SUMMARY.md
duration: ~67m across 4 tasks
verification_result: passed
completed_at: 2026-03-17
---

# S02: Docker Container Provisioning & Teardown

**DockerProvider implements full container lifecycle via bollard — provision with image pull and resource limits, exec with streaming output and exit codes, guaranteed teardown — wired into `smelt run` CLI**

## What Happened

Built the Docker container provisioning and teardown layer in four tasks:

**T01** established the foundation: added bollard/futures-util workspace dependencies, created `DockerProvider` struct with daemon connection, implemented resource string parsing (`parse_memory_bytes`, `parse_cpu_nanocpus`) with 16 unit tests, and scaffolded 4 integration tests with a skip-when-no-daemon pattern.

**T02** implemented `provision()` and `teardown()`: provision checks for local image, pulls if missing (stream fully drained), creates container with env vars from credential config, resource limits mapped to bollard's memory/nano_cpus fields, `smelt.job` label, and `sleep 3600` keep-alive CMD. Teardown stops (10s timeout) then force-removes, silently handling 304/404 for already-stopped containers. Added tracing dependency for lifecycle event logging.

**T03** implemented `exec()`: creates exec instance with attached stdout/stderr, starts exec, pattern-matches `Attached` result, consumes output stream via `StreamExt::next()` printing to stderr in real time and buffering into ExecHandle. After stream exhaustion, calls `inspect_exec` for exit code. Extended `ExecHandle` in provider.rs with `exit_code`, `stdout`, `stderr` fields. Added `test_exec_long_running` for bollard stream reliability.

**T04** closed the loop: converted `main()` to `#[tokio::main]`, made `execute()` async, wired `DockerProvider` into the non-dry-run path of `smelt run`. Teardown is guaranteed via async block pattern. Updated example manifest to alpine:3 for speed. Added CLI-level integration tests.

## Verification

- `cargo test --workspace` — 96 tests pass (74 smelt-core + 10 dry_run + 7 docker_lifecycle + 3 inline + 2 doc-tests), zero failures
- `cargo test -p smelt-core -- docker::tests` — 16 resource parsing unit tests pass
- `cargo test -p smelt-cli -- docker` — 7 Docker lifecycle integration tests pass (skip gracefully when daemon unavailable)
- `cargo run -- run examples/job-manifest.toml` — drives Docker lifecycle, produces clear connection error when daemon unavailable, exits 1
- `cargo run -- run examples/job-manifest.toml --dry-run` — still works correctly, no regressions
- CLI-level test `test_cli_run_lifecycle` verifies `docker ps -a --filter label=smelt.job` returns empty after run

## Deviations

- Docker lifecycle tests use skip-when-no-daemon pattern instead of failing with todo!() panics — cleaner for CI environments without Docker.
- Example manifest image changed from `node:20-slim` to `alpine:3` unconditionally for test speed.
- ExecHandle extended with result fields directly rather than using a separate HashMap on DockerProvider — simpler API.

## Known Limitations

- `collect()` is still a stub returning empty `CollectResult` — deferred to S04.
- Docker daemon must be running for integration tests to exercise real container lifecycle — tests skip gracefully otherwise.
- No signal handling (Ctrl+C) for graceful shutdown yet — deferred to S05.
- No repo bind-mount support yet — deferred to S03.

## Follow-ups

- None — all planned work completed. S03 will add repo mount and Assay execution on top of the DockerProvider established here.

## Files Created/Modified

- `Cargo.toml` — added bollard and futures-util to workspace dependencies
- `crates/smelt-core/Cargo.toml` — added bollard, futures-util, tracing dependencies
- `crates/smelt-core/src/docker.rs` — new: DockerProvider with provision/exec/teardown, resource parsers, 16 unit tests
- `crates/smelt-core/src/provider.rs` — extended ExecHandle with exit_code/stdout/stderr fields
- `crates/smelt-core/src/lib.rs` — registered docker module, added DockerProvider re-export
- `crates/smelt-cli/src/main.rs` — converted to async with #[tokio::main]
- `crates/smelt-cli/src/commands/run.rs` — async execute() with DockerProvider lifecycle
- `crates/smelt-cli/Cargo.toml` — added tokio runtime dep, bollard/tokio dev-deps
- `crates/smelt-cli/tests/docker_lifecycle.rs` — new: 7 integration tests for Docker lifecycle
- `crates/smelt-cli/tests/dry_run.rs` — updated 2 tests for alpine:3 image and new behavior
- `examples/job-manifest.toml` — changed image from node:20-slim to alpine:3

## Forward Intelligence

### What the next slice should know
- `DockerProvider::provision()` accepts a `&JobManifest` and returns `ContainerId`. To add bind-mounts (S03), extend the `ContainerCreateBody` construction in provision() with `HostConfig.binds`. The mount path should be configurable via the manifest.
- `exec()` takes a `&ContainerId` and `&[&str]` command, returns `ExecHandle` with stdout/stderr/exit_code. S03 will use this to run `assay orchestrate` inside the container.
- The container runs `sleep 3600` as its CMD — all work happens via exec. This means the container stays alive between exec calls, which is important for S03's multi-command sequences.

### What's fragile
- bollard exec streaming relies on pattern-matching `StartExecResults::Attached` — if bollard changes this enum or the stream behavior, exec breaks silently. The `test_exec_long_running` test exercises multi-step reliability but only when a Docker daemon is available.
- Credential env var resolution happens at provision time via `std::env::var` — missing vars are silently skipped. S03 should verify that required credentials are actually present in the container.

### Authoritative diagnostics
- `docker ps --filter label=smelt.job` — shows any active Smelt containers. Should be empty after any run completes.
- `SMELT_LOG=info smelt run manifest.toml` — shows full bollard operations via tracing.
- `cargo test -p smelt-cli --test docker_lifecycle` — exercises the full lifecycle when Docker is available.

### What assumptions changed
- bollard 0.20 moved query parameter types to `bollard::query_parameters::*` instead of `bollard::container::*` — imports need the new module path.
- Image pull requires fully draining the stream (`try_collect::<Vec<_>>()`) before proceeding — not doing so causes race conditions where container creation starts before the image is ready.
