# S03: ComposeProvider Lifecycle — Research

**Date:** 2026-03-21
**Domain:** Docker Compose subprocess management + Rust async (tokio::process), bollard, tempfile
**Confidence:** HIGH

## Summary

S03 implements `ComposeProvider: RuntimeProvider` — the full lifecycle provider that ties together `generate_compose_file()` (from S02), subprocess `docker compose` invocations, healthcheck polling, and bollard-based exec delegation. The primary risk identified in the roadmap (`docker compose ps --format json` stability and the internal state map) is well-understood after direct live testing: the JSON output is line-delimited NDJSON, the `Health` field is `""` for services without a healthcheck and `"starting"`→`"healthy"` for services with one, and state-map ownership fits cleanly into `Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>`.

The exec delegation path (S03 boundary: `exec` and `exec_streaming` delegate directly to an internal `DockerProvider`) is the most natural design — the agent container is a first-class Docker container after provisioning and bollard can target it by its container ID without any compose involvement. The `ComposeProvider` struct needs an internal `DockerProvider` field alongside the state map.

The two production dependencies that need to be promoted from dev-only to production: `tempfile` (needed to hold the compose file on disk during the entire provision→teardown lifecycle; must not be dropped early) and `serde_json` (needed for non-optional JSON parsing of `docker compose ps --format json` output — currently only enabled via the `forge` feature flag).

## Recommendation

Implement `ComposeProvider` with these exact fields and patterns:

```rust
pub struct ComposeProvider {
    docker: DockerProvider,
    state: Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>,
}

struct ComposeProjectState {
    project_name: String,
    compose_file_path: PathBuf,
    _temp_dir: TempDir,   // kept alive until teardown; Drop removes temp dir
}
```

For `provision()`:
1. Resolve credentials to `HashMap<String, String>` (same pattern as `DockerProvider::provision` — iterate `credentials.env`, read each env var)
2. Call `generate_compose_file(manifest, project_name, &extra_env)`
3. Create `TempDir`, write compose YAML to a `NamedTempFile` inside it (or directly write to a fixed filename inside the TempDir)
4. Run `docker compose -f <path> -p <project_name> up -d` via `tokio::process::Command`
5. Poll `docker compose -f <path> -p <project_name> ps --format json` until all non-agent services are healthy/running
6. Extract agent container ID from a second `ps --format json` call (find line where `"Service":"smelt-agent"`)
7. Store `ComposeProjectState` keyed by agent `ContainerId`; return agent `ContainerId`

For healthcheck polling logic (critical correctness path): for each NDJSON line from `ps --format json`, a non-agent service is "ready" when:
- `Health == ""` (no healthcheck defined) AND `State == "running"`, OR
- `Health == "healthy"` (healthcheck passed)

A service is NOT ready when `Health == "starting"` or `Health == "unhealthy"` or `State != "running"`. Print `"Waiting for <service> to be healthy..."` to stderr before the polling loop starts.

For `exec()` and `exec_streaming()`: delegate to `self.docker.exec(...)` and `self.docker.exec_streaming(...)` directly — the agent container ID is a real bollard container ID. No compose involvement needed.

For `teardown()`: look up `ComposeProjectState` by `ContainerId`, run `docker compose -f <path> -p <project_name> down --remove-orphans`, remove the entry from the state map. `ComposeProjectState._temp_dir` is dropped automatically when the state is removed from the map.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| Temp directory lifecycle tied to struct field | `tempfile::TempDir` with `Drop` impl | TempDir is deleted when dropped; store as `_temp_dir: TempDir` in `ComposeProjectState` so the temp file lives exactly as long as the project state. Do NOT use `NamedTempFile` as the primary holder — it is deleted when the file handle is dropped, not when the state is dropped. Write compose YAML to a file inside TempDir instead. |
| NDJSON parsing of `docker compose ps` output | `serde_json::from_str` per line | Output is line-delimited JSON (one JSON object per line), not a JSON array. Use `serde_json::from_str::<serde_json::Value>(line)` per non-empty line. Fields needed: `"Service"` (String), `"Health"` (String), `"State"` (String). |
| Subprocess `docker compose` invocation | `tokio::process::Command` | Already used by `git/cli.rs` in this crate. Same pattern: `.output().await`, check `.status.success()`, capture stdout/stderr. |
| Exec delegation to Docker daemon | Internal `DockerProvider` field | `DockerProvider::new()` is cheap; reuse for exec/exec_streaming; the agent container ID is a real Docker container ID that bollard can target directly. |
| Thread-safe internal state with `&self` methods | `Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>` | `RuntimeProvider` takes `&self`; state mutation requires interior mutability; `Arc<Mutex<...>>` is `Send + Sync`, which is required. |

## Existing Code and Patterns

- `crates/smelt-core/src/docker.rs` — `DockerProvider: RuntimeProvider` is the primary model. Follow its `provision`/`exec`/`teardown` pattern exactly; especially the 404-tolerant stop/remove pattern in `teardown`. The credential resolution loop (`credentials.env.iter().filter_map(|(key, env_var)| std::env::var(env_var).ok().map(|val| ...))`) is the same pattern needed to build `extra_env` for `generate_compose_file`.
- `crates/smelt-core/src/git/cli.rs` — `tokio::process::Command` usage pattern for subprocess invocation with `.output().await`, stdout/stderr capture, and `status.success()` check. This is the canonical pattern for `docker compose` subprocesses.
- `crates/smelt-core/src/compose.rs` — `generate_compose_file()` is the pure function to call at provision time. `ComposeProvider` struct is currently an empty stub — S03 fills it in. Note: `generate_compose_file` takes `extra_env: &HashMap<String, String>`, so resolve credentials before calling it.
- `crates/smelt-cli/tests/docker_lifecycle.rs` — `docker_provider_or_skip()` pattern for graceful Docker skip in tests (lines 153-160). Use the same pattern for compose integration tests: try `docker compose` availability and return early with `eprintln!` if unavailable.
- `crates/smelt-core/src/manifest.rs` — `ComposeService.extra: IndexMap<String, toml::Value>` — not directly needed in S03 (already consumed by `generate_compose_file`).

## Constraints

- `tempfile` is currently in `[dev-dependencies]` of `smelt-core/Cargo.toml`. Move it to `[dependencies]` — `TempDir` is needed in `ComposeProjectState` which lives in production code.
- `serde_json` is currently optional (behind `forge` feature) in `smelt-core/Cargo.toml`. For compose JSON parsing, it must be an unconditional production dependency. Add `serde_json = "1"` to `[dependencies]` directly (without `optional = true`), separate from the forge-gated optional entry.
- `tokio::process` is already enabled via `features = ["process"]` in workspace Cargo.toml (`tokio = { version = "1", features = ["macros", "rt-multi-thread", "process", "signal"] }`). No change needed.
- `docker compose` v2 only (confirmed: v2.40.3). Command is `docker compose` (space, not hyphen). The `-f <path> -p <project_name>` flags must appear before the subcommand (`up`, `ps`, `down`).
- `RuntimeProvider` trait uses RPITIT (D019) — `ComposeProvider::provision` must be an `async fn` returning `impl Future<Output = ...> + Send`. No `async_trait` macro.
- `#![deny(missing_docs)]` is enforced in `lib.rs`. All `pub` items need doc comments.
- The `collect()` method on `ComposeProvider` should follow `DockerProvider::collect()` — return a no-op `CollectResult` (collection is handled host-side by `ResultCollector`, not by the provider).

## Common Pitfalls

- **`TempDir` dropped too early** — If you use `let _td = TempDir::new()?; let path = _td.path().join("docker-compose.yml");`, the `_td` binding may be dropped at the end of the block. Store `TempDir` inside `ComposeProjectState` so it lives until teardown. Never store only the `PathBuf` — the temp directory is deleted when `TempDir` drops.
- **`docker compose ps --format json` is NDJSON, not a JSON array** — Output is one JSON object per line, not `[{...}, {...}]`. Parse line-by-line with `serde_json::from_str` per non-empty line. Confirmed by live testing: two services produce two separate JSON lines.
- **Services without healthcheck always have `Health: ""`** — Do NOT wait for `Health == "healthy"` on services that have no healthcheck defined. A service with no healthcheck is "ready" when `State == "running"`. Mixing these up means the polling loop hangs forever on services that will never become "healthy".
- **Agent container ID comes from `ps --format json`** — After `docker compose up -d`, the agent container ID is NOT returned directly. Extract it by calling `ps --format json` and finding the line where `Service == "smelt-agent"`, then reading the `ID` field. The `ID` field is the short Docker container ID (12 hex chars); this is a valid bollard container ID.
- **`Arc<Mutex<...>>` lock must not be held across `.await`** — Lock the mutex, clone what you need (project name, path), release the lock, then call the subprocess. Holding the lock across an `.await` point is unsound (and will cause a compile error with `std::sync::Mutex` since `MutexGuard` is not `Send`). Use a block to drop the lock before awaiting.
- **`docker compose down` error handling** — If teardown is called after a failed provision (compose up failed), the compose file and project may not exist. The `docker compose down` command may fail with a non-zero exit. Make teardown fault-tolerant: log the error but do not propagate it (consistent with D023/D038 idempotent teardown pattern).
- **Project name uniqueness** — If two jobs with the same `job.name` run concurrently, they'd conflict on the compose project name. Use `smelt-<job-name>` for simplicity in M004 (concurrent runs with same job name are not a supported scenario). A UUID suffix can be added later if needed.
- **`docker compose up -d` exits before containers are running** — `up -d` returns as soon as it starts pulling/creating containers, not when they're healthy. The polling loop is mandatory — do not assume services are ready immediately after `up -d` returns.

## Key Findings from Live Testing

**`docker compose ps --format json` output (v2.40.3, confirmed):**

Service without healthcheck (`dummy` running `sleep 3600`):
```json
{"Health":"","State":"running","Service":"dummy","ID":"7cf4c7deacd5","Names":"smelt-research-test-dummy-1",...}
```

Service with healthcheck during startup (`postgres:16-alpine`):
```json
{"Health":"starting","State":"running","Service":"postgres","ID":"15492250b81c","Names":"smelt-research-test-postgres-1",...}
```

Service with healthcheck after passing (`postgres:16-alpine`):
```json
{"Health":"healthy","State":"running","Service":"postgres","ID":"15492250b81c","Names":"smelt-research-test-postgres-1",...}
```

**Readiness logic to implement:**
```
for each non-smelt-agent service in ps output:
  if health == "" AND state == "running"  → ready (no healthcheck)
  if health == "healthy"                  → ready (healthcheck passed)
  if health == "starting"                 → not ready (still starting)
  if health == "unhealthy"                → unhealthy (should surface error)
  if state != "running"                   → not ready (still creating/stopping)
```

**`docker compose down` cleanup (confirmed):** Removes containers and network. Named volumes are retained by default. Output goes to stderr (progress lines). Command exits 0 on success.

## Integration Test Plan

Tests live in `crates/smelt-cli/tests/` as a new `compose_lifecycle.rs` file (following the `docker_lifecycle.rs` pattern). All tests skip gracefully when `docker compose` is unavailable.

Skip check: Try running `docker compose version` via `std::process::Command`; if it fails, skip with `eprintln!`.

Three integration tests:
1. **`test_compose_provision_exec_teardown`** — Provision with `alpine:3` as smelt-agent and no sidecars (empty services), exec `echo hello` in agent, verify exit code 0, teardown, confirm containers removed via `docker ps --filter label=smelt.job=<name>`.
2. **`test_compose_healthcheck_wait_postgres`** — Provision with `postgres:16-alpine` sidecar (with `pg_isready` healthcheck), agent is `alpine:3`. Verify provision completes only after postgres is healthy. Exec `ping postgres -c 1` (or equivalent connectivity check) inside agent to confirm network reachability.
3. **`test_compose_teardown_after_exec_error`** — Provision with empty services, exec a command that exits non-zero, call teardown, confirm containers are removed. Validates idempotent teardown path.

These tests use `tokio::test` with the `#[tokio::test]` attribute (same as existing integration tests in docker_lifecycle.rs).

## Open Risks

- **`serde_json` dep change may affect downstream** — Promoting `serde_json` from optional to unconditional in smelt-core could slightly increase compile time for non-forge consumers. Risk is low: `serde_json` is already compiled transitively by most Rust projects.
- **Healthcheck `unhealthy` state** — Current readiness polling should detect `unhealthy` and propagate an error rather than polling forever. Implement a max-attempts (e.g. 60 attempts × 2s = 120s) or respect `manifest` timeout.
- **Polling interval and maximum wait** — The polling loop needs a timeout. The roadmap doesn't specify one; use the manifest's `session[0].timeout` or a configurable constant (suggest 120s default, matching the existing DockerProvider assumption).

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| Rust async subprocess | — | No skills package needed — patterns from `git/cli.rs` are sufficient |
| tempfile crate | — | Covered by existing usage in this codebase |
| Docker Compose | — | No skills package; live testing was sufficient |

## Sources

- Live testing of `docker compose ps --format json` on Docker Compose v2.40.3 — confirmed NDJSON format and `Health`/`State` field values for all service states (source: local, 2026-03-21)
- `crates/smelt-core/src/docker.rs` — exec delegation pattern and teardown error handling patterns
- `crates/smelt-cli/tests/docker_lifecycle.rs` — Docker skip pattern for integration tests
- `crates/smelt-core/src/git/cli.rs` — `tokio::process::Command` subprocess pattern
