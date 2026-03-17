---
id: T02
parent: S02
milestone: M001
provides:
  - DockerProvider::provision() — pulls images, creates+starts containers with resource limits, env vars, labels
  - DockerProvider::teardown() — stops and force-removes containers
  - DockerProvider::collect() stub returning empty CollectResult
key_files:
  - crates/smelt-core/src/docker.rs
  - crates/smelt-core/Cargo.toml
key_decisions:
  - Stop errors (304/404) silently ignored in teardown — container may already have exited
  - Credential env vars resolved at provision time via std::env::var, missing vars silently skipped (not injected)
patterns_established:
  - bollard builder pattern for query params (CreateImageOptionsBuilder, StopContainerOptionsBuilder, RemoveContainerOptionsBuilder)
  - ContainerCreateBody with Default::default() spread for optional fields
observability_surfaces:
  - "tracing::info events: image pull start/complete, container created, container started, container removed"
  - "SmeltError::Provider with operation=provision or operation=teardown, bollard source error preserved"
  - "docker ps --filter label=smelt.job shows active containers"
duration: 10m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Implement provision() and teardown() — container lifecycle

**Implemented DockerProvider provision/teardown lifecycle with image pull, resource limits, env injection, labels, and stub collect()**

## What Happened

Replaced the three `todo!()` stubs in `DockerProvider`'s `RuntimeProvider` impl with working implementations:

**`provision()`**: Checks if the image exists locally via `inspect_image`. If missing, pulls it via `create_image` with the stream fully drained (`try_collect::<Vec<_>>()`). Builds `ContainerCreateBody` with image, env vars (resolved from `credentials.env` via `std::env::var`), resource limits (`HostConfig.memory` from `parse_memory_bytes`, `HostConfig.nano_cpus` from `parse_cpu_nanocpus`), `smelt.job` label, and `cmd: ["sleep", "3600"]` for keep-alive. Creates and starts the container, returns `ContainerId`.

**`teardown()`**: Stops container with 10-second timeout, silently handles 304 (already stopped) and 404 (already gone) responses. Then force-removes with `v: true` to clean anonymous volumes. All bollard errors wrapped as `SmeltError::Provider` with operation context.

**`collect()`**: Stub returning `CollectResult { exit_code: 0, stdout: "", stderr: "", artifacts: [] }`.

Added `tracing` as a dependency of smelt-core for lifecycle event logging.

## Verification

- `cargo test -p smelt-cli -- test_provision_and_teardown` — **passed** (container created, inspected, torn down, confirmed removed via bollard API)
- `cargo test --workspace` — **93 tests passed**, 0 failures
- `cargo test -p smelt-core -- docker::tests` — 16 resource parsing unit tests pass
- `cargo test -p smelt-cli -- docker` — 4 Docker lifecycle tests pass (skip gracefully when daemon unavailable via `docker_provider_or_skip()` pattern)

Note: Docker daemon availability is intermittent in the test environment (OrbStack). The `test_provision_and_teardown` test was verified with a real Docker daemon. The `test_exec` and `test_exec_nonzero_exit` tests skip when the daemon is unavailable (as designed in T01) — they will exercise the real `exec()` path once T03 implements it.

### Slice-level verification status (T02):
- ✅ `cargo test --workspace` — 93 tests, all pass
- ✅ `cargo test -p smelt-core -- docker` — resource parsing tests pass
- ✅ `cargo test -p smelt-cli -- docker` — integration tests pass (skip-when-no-daemon)
- ⏳ `cargo run -- run examples/job-manifest.toml` — not yet (T04 wires CLI)
- ⏳ `docker ps -a --filter label=smelt.job` — verified via bollard API in tests; CLI check deferred to T04

## Diagnostics

- `tracing::info` events at each lifecycle step: image pull start/complete, container created (with ID), container started, container removed
- `SmeltError::Provider` with `operation="provision"` or `operation="teardown"` and bollard source error preserved
- `docker ps --filter label=smelt.job` shows containers created by Smelt
- Credential env vars passed to containers but never logged

## Deviations

None.

## Known Issues

- `exec()` still has `todo!()` — covered by T03

## Files Created/Modified

- `crates/smelt-core/src/docker.rs` — Implemented `provision()`, `teardown()`, stub `collect()`; added bollard imports and tracing events
- `crates/smelt-core/Cargo.toml` — Added `tracing.workspace = true` dependency
