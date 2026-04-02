# S03: ComposeProvider Lifecycle

**Goal:** Implement `ComposeProvider: RuntimeProvider` with a full provision → exec → teardown lifecycle backed by `docker compose` subprocesses, bollard-delegated exec, internal `Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>` state tracking, and three integration tests that prove the lifecycle works against real Docker.
**Demo:** `cargo test --test compose_lifecycle -p smelt-cli` runs; tests skip gracefully when Docker is unavailable and pass (provision, exec, teardown verified; healthcheck wait with real Postgres confirmed) when Docker is present.

## Must-Haves

- `ComposeProvider` fields: `docker: DockerProvider`, `state: Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>`
- `ComposeProjectState { project_name: String, compose_file_path: PathBuf, _temp_dir: TempDir }` private type with `_temp_dir` keeping the directory alive until teardown
- `provision()`: resolves credentials, calls `generate_compose_file()`, writes YAML to a file inside a `TempDir`, runs `docker compose up -d`, prints `Waiting for <service> to be healthy...` to stderr before polling, polls `docker compose ps --format json` (NDJSON) until all non-agent services are healthy-or-running (max 120s), extracts agent container ID from ps output, stores `ComposeProjectState`, returns agent `ContainerId`
- `exec()` and `exec_streaming()` delegate to `self.docker` with the agent container ID — no compose involvement
- `collect()` returns a no-op `CollectResult` (collection is host-side)
- `teardown()`: looks up `ComposeProjectState`, runs `docker compose down --remove-orphans`, removes entry from state map; fault-tolerant (logs error but does not propagate if compose down fails)
- `tempfile` promoted from `[dev-dependencies]` to `[dependencies]` in `smelt-core/Cargo.toml`
- Unconditional `serde_json = "1"` added to `[dependencies]` in `smelt-core/Cargo.toml` (separate from the forge-gated optional entry)
- Integration tests in `crates/smelt-cli/tests/compose_lifecycle.rs`: skip gracefully when Docker/compose unavailable; three tests covering provision+exec+teardown, healthcheck wait with Postgres, teardown after exec error
- `cargo test --workspace` all green (zero regressions)

## Proof Level

- This slice proves: integration (real Docker daemon, real compose subprocess, real healthcheck polling)
- Real runtime required: yes — Docker daemon with `docker compose` v2
- Human/UAT required: no — all acceptance criteria are machine-verifiable

## Verification

All commands must pass:

```
# Core implementation compiles and passes smelt-core unit tests
cargo test -p smelt-core --lib 2>&1 | grep -E "(test result|FAILED)"
# → test result: ok. N passed; 0 failed

# Integration tests skip gracefully when Docker unavailable
cargo test -p smelt-cli --test compose_lifecycle 2>&1

# Full workspace — zero regressions
cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"
```

When Docker is available, all three integration tests pass:
- `test_compose_provision_exec_teardown` — provision with empty services, exec `echo hello`, exit 0, teardown, confirm no containers with `smelt.job=<name>` label remain
- `test_compose_healthcheck_wait_postgres` — provision with `postgres:16-alpine` sidecar (pg_isready healthcheck), confirm provision returns only after postgres is healthy
- `test_compose_teardown_after_exec_error` — provision with empty services, exec a failing command, call teardown, confirm containers removed

## Observability / Diagnostics

- Runtime signals: `tracing::info!` at provision start, `up -d` completion, each healthcheck poll iteration, exec delegation, teardown start/finish; `eprintln!("Waiting for {service} to be healthy...")` to stderr before polling loop starts
- Inspection surfaces: `docker compose -f <path> -p <name> ps` — shows live service state; `docker ps --filter label=smelt.job=<name>` — confirms container cleanup after teardown
- Failure visibility: healthcheck timeout surfaces as `SmeltError::Provider { operation: "provision", message: "timed out waiting for services to become healthy after 120s" }`; unhealthy state surfaces as `SmeltError::Provider { operation: "provision", message: "service <name> became unhealthy" }`; teardown errors logged via `tracing::warn!` but not propagated (D023/D038)
- Redaction constraints: credential env var values must not appear in tracing output — log key names only (consistent with existing DockerProvider pattern)

## Integration Closure

- Upstream surfaces consumed: `generate_compose_file()` from S02; `ComposeProvider` stub from S02; `ComposeService`, `JobManifest.services`, `environment.image`, `environment.runtime` from S01; `DockerProvider::new()`, `DockerProvider::exec()`, `DockerProvider::exec_streaming()` from existing `docker.rs`; `tokio::process::Command` pattern from `git/cli.rs`
- New wiring introduced in this slice: `ComposeProvider` goes from empty stub to full `RuntimeProvider` impl; `provision()` wires `generate_compose_file()` → TempDir → `docker compose up` → healthcheck polling → bollard agent ID extraction; `exec`/`exec_streaming` wire through to `DockerProvider`; `teardown()` wires to `docker compose down`
- What remains before the milestone is truly usable end-to-end: S04 dispatch in `run.rs` (`match runtime { "compose" => ComposeProvider, ... }`), `print_execution_plan()` compose section, and `examples/job-manifest-compose.toml`

## Tasks

- [x] **T01: Implement ComposeProvider struct and all RuntimeProvider methods** `est:1h30m`
  - Why: The `ComposeProvider` stub from S02 is empty — this task fills it with the full provision/exec/exec_streaming/collect/teardown implementation and its internal state type, plus the dependency changes that production code requires (`tempfile` and `serde_json`)
  - Files: `crates/smelt-core/Cargo.toml`, `crates/smelt-core/src/compose.rs`
  - Do: (1) Move `tempfile` from `[dev-dependencies]` to `[dependencies]` in `smelt-core/Cargo.toml`; (2) Change `serde_json = { version = "1", optional = true }` in `[dependencies]` to `serde_json = "1"` (remove `optional = true`), remove `"dep:serde_json"` from the `forge` feature, remove `serde_json` from `[dev-dependencies]`; (3) Add `use` imports for `tempfile::TempDir`, `tokio::process::Command`, `serde_json`, `std::sync::{Arc, Mutex}`, `std::path::PathBuf`, `tracing::{info, warn}` in `compose.rs`; (4) Define private `ComposeProjectState { project_name: String, compose_file_path: PathBuf, _temp_dir: TempDir }` with doc comment; (5) Replace `pub struct ComposeProvider {}` with fields `docker: DockerProvider` and `state: Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>`, add `pub fn new() -> crate::Result<Self>` connecting DockerProvider and creating empty state map; (6) Implement `provision()`: resolve credentials env (iterate `manifest.credentials.env`, `std::env::var` each value), call `generate_compose_file()`, create `TempDir::new()`, write YAML to `temp_dir.path().join("docker-compose.yml")`, run `docker compose -f <path> -p <project_name> up -d` via `tokio::process::Command` (check `status.success()`), print `"Waiting for <service> to be healthy..."` to stderr for each non-agent service before polling, poll `docker compose -f <path> -p <project_name> ps --format json` in a loop (max 60 × 2s = 120s), parse each non-empty line with `serde_json::from_str::<serde_json::Value>`, check `Health`/`State` per readiness logic from research, extract agent ID from ps output (`Service == "smelt-agent"` → `ID` field), store `ComposeProjectState`, return agent `ContainerId`; (7) Implement `exec()` as `self.docker.exec(container, command).await`; (8) Implement `exec_streaming()` as `self.docker.exec_streaming(container, command, output_cb).await`; (9) Implement `collect()` returning `Ok(CollectResult { exit_code: 0, stdout: String::new(), stderr: String::new(), artifacts: vec![] })`; (10) Implement `teardown()`: lock state, clone project_name and compose_file_path, release lock, run `docker compose -f <path> -p <project_name> down --remove-orphans`, log errors via `tracing::warn!` but don't propagate, remove entry from state map; (11) Keep all existing snapshot tests passing (they test `generate_compose_file()`, not `ComposeProvider`)
  - Verify: `cargo check -p smelt-core` exits 0; `cargo test -p smelt-core --lib 2>&1 | grep -E "(test result|FAILED)"` → test result: ok, 0 failed
  - Done when: `smelt-core` compiles with no errors, all 144+ existing unit tests pass, `ComposeProvider::new()` exists and `RuntimeProvider` impl covers all 5 methods

- [x] **T02: Write integration tests for the full compose lifecycle** `est:1h`
  - Why: The integration proof — real Docker, real `docker compose`, real healthcheck polling. These tests are the primary verification for R020 and the `docker compose ps` stability risk from the roadmap. Must skip gracefully when Docker unavailable.
  - Files: `crates/smelt-cli/tests/compose_lifecycle.rs`
  - Do: (1) Create `crates/smelt-cli/tests/compose_lifecycle.rs` with standard imports: `use smelt_core::compose::ComposeProvider; use smelt_core::provider::RuntimeProvider; use smelt_core::manifest::{ComposeService, CredentialConfig, Environment, JobManifest, JobMeta, MergeConfig, SessionDef}; use std::collections::HashMap; use indexmap::IndexMap;`; (2) Add skip helper `fn compose_provider_or_skip() -> Option<ComposeProvider>` — tries `ComposeProvider::new()`, also runs `std::process::Command::new("docker").args(["compose","version"]).output()` and returns `None` with `eprintln!` if either fails; (3) Add `compose_manifest_with_repo()` helper building a `JobManifest` with `runtime = "compose"`, `alpine:3` image, using `env!("CARGO_MANIFEST_DIR")` as repo path, with a services parameter; (4) Write `test_compose_provision_exec_teardown`: get provider or skip, provision manifest with no services, exec `["echo", "hello"]`, assert `handle.exit_code == 0` and `handle.stdout.contains("hello")`, call teardown, verify no containers with `smelt.job=compose-test-basic` label remain via `docker ps --filter`; (5) Write `test_compose_healthcheck_wait_postgres`: build a `ComposeService` for `postgres:16-alpine` with healthcheck extra fields (`test: ["CMD","pg_isready","-U","postgres"]`, `interval: "2s"`, `retries: 10`) via TOML string parse, provision manifest with that service and `alpine:3` agent, assert provision completes without timeout (the healthcheck wait logic proved), exec `["nc","-z","postgres","5432"]` or `["sh","-c","nc -z postgres 5432"]` in agent to verify network reachability, assert exit 0, teardown; (6) Write `test_compose_teardown_after_exec_error`: provision with no services, exec `["sh","-c","exit 1"]`, assert exit_code == 1, call teardown, verify no containers remain; (7) Pre-clean orphan containers at start of each test using `docker ps --filter label=smelt.job=<name>` + `docker rm -f` (D041 pattern); (8) `cargo test -p smelt-cli --test compose_lifecycle 2>&1` — confirm graceful skip message when Docker unavailable
  - Verify: `cargo test -p smelt-cli --test compose_lifecycle 2>&1` exits 0 (skipping or passing); `cargo test --workspace 2>&1 | grep -E "(test result|FAILED)"` shows 0 failures
  - Done when: All three tests compile and either skip gracefully (no Docker) or pass (Docker present); workspace test suite stays green

## Files Likely Touched

- `crates/smelt-core/Cargo.toml` — promote `tempfile` to deps, add unconditional `serde_json`
- `crates/smelt-core/src/compose.rs` — full `ComposeProvider` impl + `ComposeProjectState` + `RuntimeProvider` methods
- `crates/smelt-cli/tests/compose_lifecycle.rs` — new integration test file (3 tests)
