---
estimated_steps: 4
estimated_files: 1
---

# T02: Implement provision() and teardown() — container lifecycle

**Slice:** S02 — Docker Container Provisioning & Teardown
**Milestone:** M001

## Description

Implements the core container lifecycle: `provision()` pulls the image if needed, creates a container with resource limits, env vars, and labels, then starts it. `teardown()` stops and force-removes the container. Also implements the stub `collect()` returning an empty result. This is the highest-risk piece — validating that bollard's container create/start/stop/remove API works reliably.

## Steps

1. Implement `provision()` in `DockerProvider`:
   - Check if image exists locally via `inspect_image`. If not, pull via `create_image` with `CreateImageOptions { from_image: image_name }`. **Drain the entire pull stream** with `.try_collect::<Vec<_>>().await` — failing to drain means the image isn't actually ready.
   - Build `ContainerCreateBody` with: `image` from manifest `environment.image`, `env` from `credentials.env` (as `["KEY=VALUE"]` vec), `host_config` with `memory` (parsed via `parse_memory_bytes`), `nano_cpus` (parsed via `parse_cpu_nanocpus`), and `labels` (`smelt.job=<manifest.job.name>`).
   - Set `cmd` to `["sleep", "3600"]` (keep-alive; actual work runs via exec) or equivalent idle command.
   - Create container via `create_container` with `CreateContainerOptions` (no name — let Docker generate one). Start it via `start_container`.
   - Return `ContainerId` wrapping the Docker container ID.
   - Add `tracing::info!` events for image pull start/complete, container create, container start.
2. Implement `teardown()`:
   - Stop container via `stop_container` with a 10-second timeout. Ignore "not running" errors (container may have already exited).
   - Remove container via `remove_container` with `RemoveContainerOptions { force: true, v: true }` to also remove anonymous volumes.
   - Add `tracing::info!` event for container removal.
   - Wrap all bollard errors with `SmeltError::provider_with_source("teardown", ...)`.
3. Implement stub `collect()` returning `CollectResult { exit_code: 0, stdout: String::new(), stderr: String::new(), artifacts: vec![] }`.
4. Run integration tests: `cargo test -p smelt-cli -- docker_lifecycle::test_provision_and_teardown`. Verify container appears during test and is gone after.

## Must-Haves

- [ ] `provision()` pulls missing images and creates/starts containers with correct config
- [ ] Resource limits (memory, cpu) correctly mapped from manifest strings to Docker API values
- [ ] `smelt.job` label applied to containers
- [ ] Credential env vars injected into container environment
- [ ] `teardown()` force-removes containers even if already stopped
- [ ] All bollard errors wrapped as `SmeltError::Provider` with operation context

## Verification

- `cargo test -p smelt-cli -- docker_lifecycle::test_provision_and_teardown` passes
- After test: `docker ps -a --filter label=smelt.job` returns no containers
- `cargo test --workspace` — no regressions in existing tests

## Observability Impact

- Signals added/changed: `tracing::info` events for image pull, container create, container start, container teardown. `SmeltError::Provider` carries "provision"/"teardown" operation context with bollard source error.
- How a future agent inspects this: `docker ps --filter label=smelt.job` shows active Smelt containers. Tracing events at `info` level show lifecycle progression.
- Failure state exposed: Image pull failures, container create failures, and teardown failures all produce `SmeltError::Provider` with the specific bollard error as source.

## Inputs

- `crates/smelt-core/src/docker.rs` — `DockerProvider` struct and resource parsers from T01
- `crates/smelt-core/src/provider.rs` — `RuntimeProvider` trait, `ContainerId`, `CollectResult`
- `crates/smelt-core/src/manifest.rs` — `JobManifest`, `Environment`, `CredentialConfig` for reading image/resources/credentials
- S02-RESEARCH.md — bollard `ContainerCreateBody`, `HostConfig`, `CreateImageOptions`, builder patterns, stream drain gotcha

## Expected Output

- `crates/smelt-core/src/docker.rs` — working `provision()`, `teardown()`, stub `collect()` implementations; integration test `test_provision_and_teardown` passes
