---
estimated_steps: 11
estimated_files: 2
---

# T01: Implement ComposeProvider struct and all RuntimeProvider methods

**Slice:** S03 — ComposeProvider Lifecycle
**Milestone:** M004

## Description

The `ComposeProvider` stub created in S02 is an empty struct. This task replaces it with a full `RuntimeProvider` implementation backed by `docker compose` subprocesses and bollard-delegated exec. It includes: the `ComposeProjectState` internal type that holds the `TempDir` alive, the `Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>` state map, and all five trait methods (`provision`, `exec`, `exec_streaming`, `collect`, `teardown`).

The `provision()` method is the primary risk carrier for this slice — it calls `generate_compose_file()`, writes YAML to a TempDir, runs `docker compose up -d`, prints progress to stderr, and polls `docker compose ps --format json` (NDJSON, line-by-line) until all non-agent services are healthy or running, then extracts the agent container ID.

Two Cargo.toml changes are required for production code: `tempfile` must be promoted from `[dev-dependencies]` to `[dependencies]` (TempDir lives inside ComposeProjectState in production code), and an unconditional `serde_json = "1"` entry must be added to `[dependencies]` (separate from the forge-gated optional entry) for NDJSON parsing of `docker compose ps` output.

## Steps

1. In `smelt-core/Cargo.toml`:
   - Move `tempfile.workspace = true` from `[dev-dependencies]` to `[dependencies]`.
   - Change the existing `serde_json = { version = "1", optional = true }` line under `[dependencies]` to `serde_json = "1"` (remove `optional = true` — making it unconditional).
   - Remove `"dep:serde_json"` from the `forge` feature list in `[features]` — a non-optional dep does not need to be activated by a feature.
   - Remove `serde_json = "1"` from `[dev-dependencies]` — it is now available unconditionally from `[dependencies]`.
   - Net result: `serde_json` is always compiled; `forge` feature only activates `octocrab`.

2. In `compose.rs`, add imports at the top of the file (after existing `use` lines):
   ```rust
   use std::collections::HashMap;
   use std::path::PathBuf;
   use std::sync::{Arc, Mutex};
   use tempfile::TempDir;
   use tokio::process::Command;
   use tracing::{info, warn};
   use crate::provider::{CollectResult, ContainerId, ExecHandle, RuntimeProvider};
   use crate::docker::DockerProvider;
   ```

3. Define the private `ComposeProjectState` struct directly before `ComposeProvider`:
   ```rust
   /// Internal state for a provisioned Compose project.
   ///
   /// The `_temp_dir` field owns the temporary directory holding the generated
   /// `docker-compose.yml`. It is intentionally kept alive here — dropping it
   /// would delete the file before `docker compose down` can read it.
   struct ComposeProjectState {
       project_name: String,
       compose_file_path: PathBuf,
       _temp_dir: TempDir,
   }
   ```

4. Replace the empty `pub struct ComposeProvider {}` with:
   ```rust
   pub struct ComposeProvider {
       docker: DockerProvider,
       state: Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>,
   }
   ```
   Add a `pub fn new() -> crate::Result<Self>` constructor that calls `DockerProvider::new()` and initializes an empty `Arc<Mutex<HashMap::new()>>`.

5. Implement `provision()` on `ComposeProvider`. Key correctness requirements:
   - Build `project_name = format!("smelt-{}", manifest.job.name)`.
   - Resolve credentials: `manifest.credentials.env.iter().filter_map(|(key, env_var)| std::env::var(env_var).ok().map(|val| (key.clone(), val))).collect::<HashMap<String, String>>()`.
   - Call `generate_compose_file(manifest, &project_name, &extra_env)?`.
   - Create `TempDir::new()` (`tempfile::TempDir::new().map_err(...)?`), write YAML to `temp_dir.path().join("docker-compose.yml")` using `std::fs::write`.
   - Before polling: print to stderr for each non-agent service: `eprintln!("Waiting for {} to be healthy...", svc.name)`.
   - Run `docker compose -f <compose_file_path> -p <project_name> up -d` via `tokio::process::Command`; check `status.success()`.
   - Poll `docker compose -f <path> -p <project_name> ps --format json` up to 60 times (2-second sleep between attempts). For each non-empty line parse as `serde_json::Value`; extract `Service` (String), `Health` (String), `State` (String). Readiness: (`Health == ""` AND `State == "running"`) OR `Health == "healthy"`. Error: `Health == "unhealthy"`. Not ready: anything else (including `Health == "starting"`). If any service is unhealthy, return error. If all non-agent services are ready, proceed.
   - Extract agent container ID: after health check passes, call ps again (or reuse last output), find the line where `Service == "smelt-agent"`, extract `ID` field (short container ID — valid bollard ID).
   - Store `ComposeProjectState { project_name, compose_file_path: compose_file_path.to_owned(), _temp_dir }` in the state map keyed by agent `ContainerId`.
   - Return agent `ContainerId`.
   - Lock discipline: lock the mutex only to store/retrieve state (never across `.await` points).

6. Implement `exec()` as:
   ```rust
   async fn exec(&self, container: &ContainerId, command: &[String]) -> crate::Result<ExecHandle> {
       self.docker.exec(container, command).await
   }
   ```

7. Implement `exec_streaming()` as:
   ```rust
   async fn exec_streaming<F>(&self, container: &ContainerId, command: &[String], output_cb: F) -> crate::Result<ExecHandle>
   where F: FnMut(&str) + Send + 'static {
       self.docker.exec_streaming(container, command, output_cb).await
   }
   ```

8. Implement `collect()` as a no-op:
   ```rust
   async fn collect(&self, _container: &ContainerId, _manifest: &JobManifest) -> crate::Result<CollectResult> {
       Ok(CollectResult { exit_code: 0, stdout: String::new(), stderr: String::new(), artifacts: vec![] })
   }
   ```

9. Implement `teardown()`:
   - Lock the mutex, clone `project_name` and `compose_file_path`, release lock immediately.
   - Run `docker compose -f <path> -p <project_name> down --remove-orphans`. If command fails, log via `warn!` but do NOT return an error (D023/D038 idempotent teardown).
   - Lock mutex again, remove the `ContainerId` entry (this drops `_temp_dir`, deleting the temp directory).

10. Update the `smoke_empty_services_compiles` test to exercise `ComposeProvider::new()` (it currently just constructs `ComposeProvider {}` — change it to call `ComposeProvider::new()`; wrap in a function that returns a `Result` or use `.ok()` to handle potential daemon-absent case gracefully in unit tests).

11. Verify: `cargo check -p smelt-core` exits 0; `cargo test -p smelt-core --lib 2>&1 | grep -E "(test result|FAILED)"` → 0 failed.

## Must-Haves

- [ ] `tempfile` is in `[dependencies]` (not just `[dev-dependencies]`) in `smelt-core/Cargo.toml`
- [ ] `serde_json = "1"` is an unconditional production dep in `smelt-core/Cargo.toml`
- [ ] `ComposeProjectState` private struct with `project_name`, `compose_file_path`, `_temp_dir: TempDir` fields exists in `compose.rs`
- [ ] `ComposeProvider` has `docker: DockerProvider` and `state: Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>` fields
- [ ] `ComposeProvider::new()` constructs a `DockerProvider` and empty state map, returns `crate::Result<Self>`
- [ ] `provision()` prints `"Waiting for <service> to be healthy..."` to stderr before the polling loop
- [ ] `provision()` polls `docker compose ps --format json` line-by-line (NDJSON), correctly distinguishes `Health == ""` + `State == "running"` (no-healthcheck ready) from `Health == "healthy"` (healthcheck ready) from `Health == "starting"` (not ready) from `Health == "unhealthy"` (error)
- [ ] `provision()` stores `ComposeProjectState` and returns the agent container ID
- [ ] `exec()` and `exec_streaming()` delegate directly to `self.docker`
- [ ] `teardown()` is fault-tolerant: runs `compose down --remove-orphans`, logs errors without propagating
- [ ] `cargo check -p smelt-core` exits 0
- [ ] `cargo test -p smelt-core --lib` — all existing tests pass, 0 regressions

## Verification

- `cargo check -p smelt-core` — exits 0 (no compile errors)
- `cargo test -p smelt-core --lib 2>&1 | grep -E "(test result|FAILED)"` — `test result: ok. N passed; 0 failed`
- `grep -n 'ComposeProjectState\|fn provision\|fn teardown\|fn new' crates/smelt-core/src/compose.rs` — all four lines present
- `grep 'tempfile' crates/smelt-core/Cargo.toml | head -5` — `tempfile` appears under `[dependencies]`, not only `[dev-dependencies]`
- `grep 'serde_json' crates/smelt-core/Cargo.toml` — a non-optional `serde_json = "1"` line appears under `[dependencies]`

## Observability Impact

- Signals added/changed: `tracing::info!` at provision milestones (up start, healthcheck poll, agent ID found, teardown); `eprintln!` to stderr for healthcheck wait messages; `tracing::warn!` on teardown errors
- How a future agent inspects this: `docker compose -f <path> -p <name> ps` during provision; `docker ps --filter label=smelt.job=<name>` after teardown; `SmeltError::Provider { operation: "provision", message: "timed out..." }` on healthcheck timeout
- Failure state exposed: timeout error includes operation name and 120s duration; unhealthy error includes service name; teardown errors are warned but not fatal (consistent with D023)

## Inputs

- `crates/smelt-core/src/compose.rs` — `ComposeProvider {}` stub + `generate_compose_file()` from S02 (call this at provision time)
- `crates/smelt-core/src/docker.rs` — `DockerProvider` for constructor + exec delegation; credential resolution pattern from `provision()`
- `crates/smelt-core/src/git/cli.rs` — `tokio::process::Command` usage pattern (subprocess invocation with `.output().await`, status check, stdout capture)
- `crates/smelt-core/src/provider.rs` — `RuntimeProvider` trait, `ContainerId`, `ExecHandle`, `CollectResult` types
- S03-RESEARCH.md — `ComposeProjectState` design, NDJSON parsing strategy, lock-across-await pitfall, readiness logic table

## Expected Output

- `crates/smelt-core/Cargo.toml` — `tempfile` under `[dependencies]`; unconditional `serde_json = "1"` under `[dependencies]`
- `crates/smelt-core/src/compose.rs` — `ComposeProjectState` private struct; `ComposeProvider` with two fields + `new()` constructor; full `RuntimeProvider` impl with all 5 methods; all prior snapshot tests still passing
