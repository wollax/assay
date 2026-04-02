# S02: Docker Container Provisioning & Teardown — Research

**Date:** 2026-03-17

## Summary

S02 implements `DockerProvider` against the existing `RuntimeProvider` trait using the bollard crate (v0.20.2). The bollard API is well-suited: it provides async create/start/exec/stop/remove operations with streaming output via `futures_util::Stream<Item = Result<LogOutput, Error>>`. The exec API returns attached output streams that can be forwarded to the terminal in real time, and `inspect_exec` provides exit codes after completion.

The primary risk — bollard exec reliability for long-running processes — is retired in this slice by running a multi-step command sequence inside a real container and streaming output. The container lifecycle is straightforward: pull image → create container (with env vars, resource limits, labels) → start → exec commands → stop → remove (force). Teardown must be guaranteed even on failure, which means a guard/drop pattern or explicit cleanup in every error path.

Resource limits map cleanly: `Environment.resources["memory"]` → `HostConfig.memory` (bytes), `resources["cpu"]` → `HostConfig.nano_cpus` (nanoseconds of CPU time). Bind mounts are specified via `HostConfig.binds` as `["host_path:container_path:mode"]` strings — this is deferred to S03 but the mount plumbing should be designed into `provision()` now.

## Recommendation

Build `DockerProvider` as a struct holding a `bollard::Docker` client and implementing `RuntimeProvider`. For this slice, implement `provision()`, `exec()`, and `teardown()` — `collect()` stays as a stub returning empty results (it's S04's concern). Wire the CLI `smelt run` (without `--dry-run`) to instantiate `DockerProvider` and run the lifecycle. Integration tests should use a real Docker daemon with a lightweight image (alpine:3).

Use labels (`smelt.job=<name>`) on containers for identification and cleanup. Parse resource strings ("4G" → bytes, "2" cpus → nanocpus) in a small utility module. Handle image pull as part of `provision()` — if the image isn't present locally, pull it before creating the container.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Docker API client | `bollard` 0.20.2 | De facto async Rust Docker client; D005 mandates it |
| Async streaming output | `futures-util` `StreamExt` | Already a bollard transitive dep; needed to consume `LogOutput` streams |
| Byte manipulation for log output | `bytes` crate | Already a bollard transitive dep; `LogOutput` variants carry `Bytes` |
| Resource string parsing (e.g. "4G" → bytes) | Hand-roll a small parser | Simple enough — "4G", "512M", "2" patterns only. No crate needed. |

## Existing Code and Patterns

- `crates/smelt-core/src/provider.rs` — `RuntimeProvider` trait with `provision()`, `exec()`, `collect()`, `teardown()`. `ContainerId(String)` wraps Docker container IDs. `ExecHandle` has `container: ContainerId` + `exec_id: String`. `CollectResult` has `exit_code`, `stdout`, `stderr`, `artifacts`.
- `crates/smelt-core/src/manifest.rs` — `Environment` has `runtime`, `image`, `resources: HashMap<String, String>`. `CredentialConfig` has `provider`, `model`, `env: HashMap<String, String>`. These feed directly into container config.
- `crates/smelt-core/src/error.rs` — `SmeltError::Provider { operation, message, source }` with `provider()` and `provider_with_source()` constructors. All bollard errors should be wrapped with these.
- `crates/smelt-cli/src/commands/run.rs` — Currently returns exit 1 for non-dry-run. This is where `DockerProvider` gets instantiated and the lifecycle is driven.
- `crates/smelt-core/src/lib.rs` — Module registry. New `docker.rs` module needs to be added here.
- `examples/job-manifest.toml` — Valid manifest using `runtime = "docker"`, `image = "node:20-slim"`, resource limits. Use for manual smoke testing.

## Constraints

- **Rust 2024 edition** — `RuntimeProvider` uses RPITIT (return-position impl trait in trait). The `DockerProvider` impl must match `Send` bounds on returned futures, which means all `&self` borrows across `.await` must be `Send`-safe.
- **bollard default features** include `http` + `pipe` (Unix socket + named pipe). No `ssl` needed for local Docker daemon. Keep defaults.
- **tokio runtime** — bollard requires tokio. The workspace already has `tokio` with `macros`, `rt-multi-thread`, `process`, `signal`. The CLI `main` needs `#[tokio::main]` — check if it's already async or needs conversion.
- **Docker daemon required for integration tests** — Tests that call bollard need a running Docker daemon. Must be gated or clearly documented. CI may not have Docker available.
- **`deny_unknown_fields`** on manifest structs — Adding any new fields to manifest types for Docker-specific config requires schema changes. S02 should not need new manifest fields; `Environment.resources` and `CredentialConfig.env` already provide what's needed.
- **`futures-util`** needs to be a direct dependency of smelt-core (for `StreamExt` on exec output streams), even though it's a transitive dep of bollard.

## Common Pitfalls

- **Leaking containers on error paths** — If `exec()` panics or the process is killed, containers remain running. Use a cleanup guard pattern: track active container IDs and ensure `teardown()` runs in all paths including `Drop`. For Ctrl+C, tokio's signal handling (already in workspace deps) should trigger graceful cleanup — but that's S05's scope. For S02, at minimum ensure the happy-path and error-path both call `teardown()`.
- **Image pull blocking provision** — `create_image` (pull) returns a stream that must be fully consumed before the image is ready. Forgetting to `.try_collect()` or drain the stream means the image isn't actually pulled. The bollard exec example shows the pattern: `.try_collect::<Vec<_>>().await?`.
- **Exec exit code not in stream** — The `start_exec` output stream gives stdout/stderr but NOT the exit code. Must call `inspect_exec` after the stream is fully consumed to get `exit_code`. This is a common bollard gotcha.
- **`ContainerCreateBody` not `ContainerConfig`** — bollard 0.20 uses `ContainerCreateBody` (from bollard-stubs), not the older `ContainerConfig`. The examples confirm this.
- **Resource limit units** — Docker's `Memory` field is in bytes (i64). `NanoCpus` is nanoseconds of CPU time per second (1 CPU = 1_000_000_000). The manifest uses human strings like "4G" and "2" — need a parser that converts these correctly.
- **Builder pattern for options** — bollard 0.20 uses `*OptionsBuilder::default().field(val).build()` for query parameters (e.g., `CreateContainerOptionsBuilder`, `RemoveContainerOptionsBuilder`). The older struct-literal style still works for some types but not all.
- **`start_exec` Attached variant** — Must pattern-match `StartExecResults::Attached { output, input }`. The `output` is a pinned stream. The `input` is for stdin — ignore it for non-interactive exec. Matching `Detached` would be a bug for attached execution.

## Open Risks

- **bollard exec stream reliability for long-running processes** — This is the primary risk being retired. If the stream drops or hangs for processes running >5 minutes, the fallback is shelling out to `docker exec` via `tokio::process::Command`. The integration test should include a sleep-based long-running command to exercise this.
- **Docker socket permissions** — `Docker::connect_with_socket_defaults()` requires access to `/var/run/docker.sock`. On macOS with Docker Desktop this works by default. On Linux, the user must be in the `docker` group. This isn't something Smelt can fix, but should produce a clear error.
- **Image pull authentication** — Private registries require auth. bollard's `create_image` accepts credentials, but S02 uses public images only. Private registry auth is a follow-up concern.
- **`run.rs` async conversion** — The current `execute()` function in `run.rs` is synchronous. Calling bollard requires async. The CLI main needs `#[tokio::main]` and the run command needs to be async. This is a small but necessary change.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Docker | `sickn33/antigravity-awesome-skills@docker-expert` | available (6.3K installs) — general Docker expertise, may help with container lifecycle patterns |
| Rust async | `wshobson/agents@rust-async-patterns` | available (4.4K installs) — could help with async stream handling patterns |
| Rust general | `apollographql/skills@rust-best-practices` | available (2.7K installs) — general Rust patterns |

None are directly specific to bollard or Rust Docker clients. The bollard API is well-documented in its examples and the codebase patterns from S01 provide sufficient guidance. Installing these skills is optional.

## Sources

- bollard 0.20.2 `examples/exec.rs` — Container create, exec with attached stream, force remove pattern (source: local cargo registry)
- bollard 0.20.2 `src/exec.rs` — `create_exec`, `start_exec` (returns `StartExecResults::Attached`), `inspect_exec` (returns `ExecInspectResponse` with `exit_code`) API surface (source: local cargo registry)
- bollard 0.20.2 `src/container.rs` — `create_container`, `start_container`, `stop_container`, `remove_container`, `wait_container` signatures (source: local cargo registry)
- bollard-stubs 1.52.1 `models.rs` — `ContainerCreateBody` (image, env, cmd, host_config), `HostConfig` (memory, nano_cpus, binds, auto_remove), `ExecInspectResponse` (exit_code, running) (source: local cargo registry)
- bollard-stubs 1.52.1 `query_parameters.rs` — Builder pattern for `CreateContainerOptions`, `RemoveContainerOptions`, `StartContainerOptions` (source: local cargo registry)
- S01 summary and forward intelligence — `RuntimeProvider` trait shape, `ExecHandle` limitations, RPITIT constraint (source: `.kata/milestones/M001/slices/S01/S01-SUMMARY.md`)
