---
id: T01
parent: S03
milestone: M004
provides:
  - ComposeProjectState private struct (project_name, compose_file_path, _temp_dir: TempDir)
  - ComposeProvider with docker: DockerProvider + state: Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>
  - ComposeProvider::new() constructor
  - provision() with compose up, NDJSON healthcheck polling, credential resolution, stderr progress messages
  - exec() and exec_streaming() delegating to DockerProvider
  - collect() no-op returning CollectResult with exit_code 0
  - teardown() running compose down --remove-orphans, fault-tolerant (D023/D038)
  - tempfile promoted to [dependencies] in smelt-core/Cargo.toml
  - serde_json made unconditional dep in smelt-core/Cargo.toml (was optional, forge-gated)
key_files:
  - crates/smelt-core/src/compose.rs
  - crates/smelt-core/Cargo.toml
key_decisions:
  - "provision() polls up to 60×2s (120s total) via docker compose ps --format json NDJSON; readiness: (Health=="" AND State=="running") OR Health=="healthy"; unhealthy is immediate error"
  - "Lock discipline: mutex held only for HashMap insert/remove, never across .await points"
  - "When there are no non-agent services the polling loop exits vacuously; a final separate ps call captures the agent container ID"
  - "serde_json made fully unconditional (removed from forge feature) because NDJSON parsing is production code, not forge-only"
  - "tempfile promoted from dev-dependencies to dependencies because TempDir lives inside ComposeProjectState (production struct)"
patterns_established:
  - "Compose exec delegation pattern: ComposeProvider.exec/exec_streaming delegate directly to self.docker"
  - "Fault-tolerant teardown pattern: compose down errors are logged via warn! but not propagated (consistent with D023)"
observability_surfaces:
  - "tracing::info! at provision start, up -d complete, each healthcheck poll attempt, provision complete, teardown start/finish"
  - "eprintln!(\"Waiting for {service} to be healthy...\") to stderr for each non-agent service before polling"
  - "tracing::warn! on teardown errors (compose down non-zero or spawn failure)"
  - "Failure shapes: SmeltError::Provider { operation: \"provision\", message: \"timed out waiting for services to become healthy after 120s\" }; SmeltError::Provider { operation: \"provision\", message: \"service {name} became unhealthy\" }"
duration: 25min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T01: Implement ComposeProvider struct and all RuntimeProvider methods

**Full `RuntimeProvider` impl for `ComposeProvider`: compose up + NDJSON healthcheck polling + bollard-delegated exec + fault-tolerant teardown, backed by a `TempDir`-anchored `ComposeProjectState`.**

## What Happened

Replaced the empty `ComposeProvider {}` stub from S02 with a complete `RuntimeProvider` implementation. The key changes:

**Cargo.toml:** `tempfile` was moved from `[dev-dependencies]` to `[dependencies]` because `TempDir` now lives in `ComposeProjectState` (a production struct). `serde_json` was changed from `{ version = "1", optional = true }` to `"1"` (unconditional) because NDJSON parsing of `docker compose ps` output is production code, not forge-only. The `"dep:serde_json"` activation was removed from the `forge` feature accordingly.

**compose.rs:**
- `ComposeProjectState` private struct holds `project_name: String`, `compose_file_path: PathBuf`, `_temp_dir: TempDir`. The `_temp_dir` field intentionally anchors the temp directory lifetime to the state entry — dropping it on teardown deletes the file after `compose down` has read it.
- `ComposeProvider` gained two fields: `docker: DockerProvider` and `state: Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>`. The constructor calls `DockerProvider::new()` and initialises an empty map.
- `provision()` resolves credentials from env vars, generates the compose YAML via `generate_compose_file()`, writes it to a `TempDir`, prints wait messages to stderr per service, runs `docker compose up -d`, then polls `docker compose ps --format json` (NDJSON) up to 60 times at 2s intervals. Readiness logic: `Health == "" && State == "running"` (no healthcheck container) or `Health == "healthy"` (healthcheck passed). Unhealthy is an immediate error. Timeout is 120s. The agent container ID is extracted from the `ID` field of the `smelt-agent` line.
- `exec()` and `exec_streaming()` delegate directly to `self.docker`.
- `collect()` returns an empty `CollectResult` (no-op, consistent with DockerProvider).
- `teardown()` retrieves `project_name`/`compose_file_path` from the state map (lock held briefly, never across await), runs `compose down --remove-orphans`, logs but does not propagate errors (D023/D038), then removes the state entry (which drops `_temp_dir`).
- The `smoke_empty_services_compiles` test was updated to call `ComposeProvider::new().ok()` instead of constructing the struct directly.

## Verification

```
$ cargo check -p smelt-core
Finished `dev` profile [unoptimized + debuginfo] — 0 errors

$ cargo test -p smelt-core --lib 2>&1 | grep -E "(test result|FAILED)"
test result: ok. 138 passed; 0 failed; 0 ignored

$ grep -n 'ComposeProjectState\|fn provision\|fn teardown\|fn new' crates/smelt-core/src/compose.rs
38: struct ComposeProjectState
53: state: Arc<Mutex<...>>
60: pub fn new() -> crate::Result<Self>
70: async fn provision(...)
329: ComposeProjectState { ... }
373: async fn teardown(...)

$ grep 'tempfile' crates/smelt-core/Cargo.toml
tempfile.workspace = true   ← under [dependencies]

$ grep 'serde_json' crates/smelt-core/Cargo.toml
serde_json = "1"            ← unconditional under [dependencies]
```

All 12 must-haves confirmed.

## Diagnostics

- `docker compose -f <path> -p <project> ps` — shows live service health state during provision
- `docker ps --filter label=smelt.job=<name>` — verifies container cleanup after teardown
- `tracing::info!` events emitted at: provision start, up -d completion, each healthcheck poll (with attempt counter), provision complete with agent container ID, teardown start, teardown complete
- Error shapes for future agents:
  - Timeout: `SmeltError::Provider { operation: "provision", message: "timed out waiting for services to become healthy after 120s" }`
  - Unhealthy: `SmeltError::Provider { operation: "provision", message: "service postgres became unhealthy" }`
  - Teardown errors: `warn!` only, never propagated

## Deviations

The loop exit when there are no non-agent services required a small structural adjustment: when `non_agent_services` is empty the `all_ready` check is vacuously true on the first iteration, but the loop still waits 2s for `up -d` to start the agent container. To handle the case where the agent container ID wasn't seen in that first poll (race condition), a fallback separate `ps` call was added after the loop. This is a correctness improvement over the plan, not a regression.

## Known Issues

None. Integration tests (T02) will exercise the full lifecycle against real Docker.

## Files Created/Modified

- `crates/smelt-core/src/compose.rs` — Full `ComposeProvider` implementation with all 5 `RuntimeProvider` methods, `ComposeProjectState` struct, updated `smoke_empty_services_compiles` test
- `crates/smelt-core/Cargo.toml` — `tempfile` → `[dependencies]`; `serde_json` made unconditional; `forge` feature simplified to `["dep:octocrab"]`
