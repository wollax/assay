# S02: Docker Container Provisioning & Teardown

**Goal:** `DockerProvider` implements `RuntimeProvider` using bollard, and `smelt run manifest.toml` provisions a real Docker container, execs a health-check command with streamed output, and tears down the container — even on failure.
**Demo:** Run `smelt run examples/job-manifest.toml` against a real Docker daemon. A container appears in `docker ps` during execution, a health-check command runs inside it with output streamed to the terminal, and the container is gone after completion.

## Must-Haves

- `DockerProvider` struct implementing `provision()`, `exec()`, `teardown()` (plus stub `collect()`) against `RuntimeProvider` trait
- Image pull (with stream drain) as part of `provision()`
- Container creation with env vars, resource limits (memory bytes, nanocpus), and `smelt.job` label
- `exec()` returns exit code via `inspect_exec` after streaming stdout/stderr
- `teardown()` force-removes the container — called on both success and error paths
- Resource string parsing: "4G" → bytes, "512M" → bytes, "2" cpus → nanocpus
- `smelt run` (without `--dry-run`) drives the full lifecycle through `DockerProvider`
- CLI main converted to async (`#[tokio::main]`)
- Integration tests exercise real Docker daemon with alpine:3 image

## Proof Level

- This slice proves: integration (real Docker daemon, real containers, real exec)
- Real runtime required: yes (Docker daemon must be running)
- Human/UAT required: no

## Verification

- `cargo test --workspace` — all existing tests (71) plus new tests pass, zero warnings
- `cargo test -p smelt-core -- docker` — DockerProvider unit tests (resource parsing) pass
- `cargo test -p smelt-cli -- docker` — integration tests against real Docker daemon pass (provision → exec → teardown lifecycle)
- `cargo run -- run examples/job-manifest.toml` — runs against Docker, streams output, exits cleanly with container removed
- `docker ps -a --filter label=smelt.job` — returns no containers after any test or run

## Observability / Diagnostics

- Runtime signals: `tracing::info` events for each lifecycle phase (image pull, container create, exec start, exec complete with exit code, teardown). `SmeltError::Provider` variants carry operation name and bollard error as source.
- Inspection surfaces: `docker ps --filter label=smelt.job` shows active Smelt containers. Container labels include `smelt.job=<name>` for identification.
- Failure visibility: Provider errors include operation context ("provision", "exec", "teardown", "image_pull") and the underlying bollard error message. Exec results include exit code, stdout, and stderr.
- Redaction constraints: Credential env vars are passed through to containers but never logged. `tracing` events log container IDs and image names only.

## Integration Closure

- Upstream surfaces consumed: `provider.rs` → `RuntimeProvider` trait, `ContainerId`, `ExecHandle`, `CollectResult`; `manifest.rs` → `JobManifest`, `Environment`, `CredentialConfig`; `error.rs` → `SmeltError::Provider` constructors; `commands/run.rs` → CLI entrypoint
- New wiring introduced in this slice: `DockerProvider` instantiated in `run.rs`, CLI main converted to async, full provision→exec→teardown lifecycle driven from `smelt run`
- What remains before the milestone is truly usable end-to-end: S03 (repo mount + Assay execution), S04 (result collection + branch output), S05 (monitoring + timeout + graceful shutdown), S06 (integration)

## Tasks

- [x] **T01: Add bollard dependency, resource parser, and integration test scaffolding** `est:30m`
  - Why: Establishes the foundation — bollard/futures-util deps, the `docker` module with `DockerProvider` struct, resource string parsing utilities, and failing integration tests that define what "done" looks like for S02.
  - Files: `Cargo.toml`, `crates/smelt-core/Cargo.toml`, `crates/smelt-core/src/docker.rs`, `crates/smelt-core/src/lib.rs`, `crates/smelt-cli/tests/docker_lifecycle.rs`, `crates/smelt-cli/Cargo.toml`
  - Do: Add `bollard` and `futures-util` to workspace deps. Create `docker.rs` with `DockerProvider` struct holding `bollard::Docker` client, constructor, and resource parsing fns (`parse_memory_bytes`, `parse_cpu_nanocpus`). Register module in `lib.rs`. Write integration test file with tests that assert on the full lifecycle (provision creates container, exec runs command and returns output, teardown removes container) — these tests should compile but fail since methods aren't implemented yet. Add `tokio` and `bollard` to smelt-cli dev-deps for async integration tests.
  - Verify: `cargo test -p smelt-core -- docker::tests` passes resource parsing tests. `cargo test -p smelt-cli -- docker` compiles but lifecycle tests fail (methods unimplemented).
  - Done when: `DockerProvider::new()` connects to Docker daemon, resource parsing is correct with unit tests, integration test file exists and compiles.

- [x] **T02: Implement provision() and teardown() — container lifecycle** `est:40m`
  - Why: The core container lifecycle — creates a real Docker container from a manifest and guarantees cleanup. This is the highest-risk piece (bollard API surface, image pull stream drain, resource limit mapping).
  - Files: `crates/smelt-core/src/docker.rs`
  - Do: Implement `provision()`: check if image exists locally, pull if missing (drain the `create_image` stream fully), create container with `ContainerCreateBody` (image, env vars from credentials.env, resource limits via `HostConfig.memory`/`HostConfig.nano_cpus`, labels `smelt.job=<name>`), start container, return `ContainerId`. Implement `teardown()`: stop container (10s timeout), then force-remove. Implement stub `collect()` returning empty `CollectResult`. All bollard errors wrapped with `SmeltError::provider_with_source()`. Add tracing events for each step.
  - Verify: `cargo test -p smelt-cli -- docker_lifecycle::test_provision_and_teardown` passes — container is created, visible in Docker, then removed.
  - Done when: `provision()` creates a running container from alpine:3 with correct labels and env vars, `teardown()` removes it, no orphaned containers.

- [x] **T03: Implement exec() with streaming output and exit code** `est:35m`
  - Why: Exec is the primary interface for running commands inside containers and the key risk being retired (bollard exec reliability). Must handle attached output streams and retrieve exit codes via `inspect_exec`.
  - Files: `crates/smelt-core/src/docker.rs`
  - Do: Implement `exec()`: create exec instance with `create_exec` (attach stdout/stderr), start exec with `start_exec`, match `StartExecResults::Attached`, consume the output stream printing stdout/stderr to terminal via `tracing::info` or direct `eprintln`, after stream completes call `inspect_exec` to get exit code, return `ExecHandle`. Handle non-zero exit codes by still returning successfully (caller decides policy). Add a test with a multi-step command sequence and a test with a command that runs for several seconds to exercise stream reliability.
  - Verify: `cargo test -p smelt-cli -- docker_lifecycle::test_exec` passes — command runs inside container, output is captured, exit code is correct. `cargo test -p smelt-cli -- docker_lifecycle::test_exec_nonzero_exit` passes — non-zero exit code is reported.
  - Done when: `exec()` runs commands inside a provisioned container, streams output, and correctly reports exit codes for both success and failure cases.

- [x] **T04: Wire DockerProvider into CLI and run full lifecycle integration test** `est:30m`
  - Why: Closes the loop — `smelt run manifest.toml` (without `--dry-run`) drives the full lifecycle through `DockerProvider`. The CLI main must become async, and `run.rs` must instantiate the provider and call provision→exec→teardown.
  - Files: `crates/smelt-cli/src/main.rs`, `crates/smelt-cli/src/commands/run.rs`, `crates/smelt-cli/tests/docker_lifecycle.rs`
  - Do: Convert `main()` to `#[tokio::main] async fn main()`. Make `execute()` in `run.rs` async. For non-dry-run: instantiate `DockerProvider::new()`, load+validate manifest, call `provision()`, exec a health-check command (`echo "smelt health check"` or `CMD` from manifest), stream output, call `teardown()` in both success and error paths (use a scopeguard or explicit match). Add CLI-level integration test: `smelt run examples/job-manifest.toml` succeeds, output contains expected health check text, no containers remain after.
  - Verify: `cargo test --workspace` — all tests pass (existing 71 + new docker tests). `cargo run -- run examples/job-manifest.toml` provisions, execs, streams, and tears down. `docker ps -a --filter label=smelt.job` returns empty.
  - Done when: `smelt run manifest.toml` drives the full Docker lifecycle from the CLI, the bollard exec risk is retired, and all containers are cleaned up.

## Files Likely Touched

- `Cargo.toml` — add bollard, futures-util to workspace deps
- `crates/smelt-core/Cargo.toml` — add bollard, futures-util deps
- `crates/smelt-core/src/docker.rs` — new: DockerProvider, resource parsing, RuntimeProvider impl
- `crates/smelt-core/src/lib.rs` — register docker module + re-export DockerProvider
- `crates/smelt-cli/src/main.rs` — convert to async main
- `crates/smelt-cli/src/commands/run.rs` — wire DockerProvider, async execute
- `crates/smelt-cli/Cargo.toml` — add tokio, bollard to dev-deps for async tests
- `crates/smelt-cli/tests/docker_lifecycle.rs` — new: integration tests for Docker lifecycle
