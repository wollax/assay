---
id: S03
parent: M004
milestone: M004
provides:
  - ComposeProjectState private struct (project_name, compose_file_path, _temp_dir: TempDir) in smelt-core
  - ComposeProvider with docker: DockerProvider + state: Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>
  - ComposeProvider::new() constructor connecting to local Docker daemon
  - provision(): credential resolution + TempDir YAML write + compose up -d + NDJSON healthcheck polling (60×2s, 120s total) + agent container ID extraction
  - exec() and exec_streaming() delegating to DockerProvider
  - collect() no-op returning empty CollectResult
  - teardown() running compose down --remove-orphans, fault-tolerant logging (D023/D038)
  - tempfile promoted from [dev-dependencies] to [dependencies] in smelt-core/Cargo.toml
  - serde_json made unconditional dep in smelt-core/Cargo.toml (was optional/forge-gated)
  - indexmap added to smelt-cli [dev-dependencies] for integration test helpers
  - compose_lifecycle.rs integration tests: 3 tests (provision+exec+teardown, Postgres healthcheck wait, teardown after exec error)
  - Bug fix: smelt-agent service gets command: [sleep, "3600"] in generated YAML (D083)
  - Bug fix: custom named network removed; rely on Docker Compose default project network (D082)
  - Updated 6 snapshot tests in smelt-core to match corrected compose YAML shape
requires:
  - slice: S01
    provides: ComposeService type, JobManifest.services, environment.image, environment.runtime
  - slice: S02
    provides: generate_compose_file() pure function, ComposeProvider stub in compose.rs
affects:
  - S04
key_files:
  - crates/smelt-core/src/compose.rs
  - crates/smelt-core/Cargo.toml
  - crates/smelt-cli/tests/compose_lifecycle.rs
  - crates/smelt-cli/Cargo.toml
key_decisions:
  - "D079: serde_json made unconditional dep — NDJSON parsing of docker compose ps output is production code, not forge-only"
  - "D080: tempfile promoted to production dep — TempDir lives in ComposeProjectState (production struct, outlives the provision→teardown lifecycle)"
  - "D081: healthcheck timeout 60×2s=120s; unhealthy state is immediate error; starting state waits; no manifest field"
  - "D082: smelt-agent uses Docker Compose default project network — custom named network isolated agent from user services, breaking DNS; default network gives all services shared DNS automatically"
  - "D083: smelt-agent compose service always has command: [sleep, '3600'] — alpine:3 exits immediately without it; exited containers disappear from docker compose ps output, making agent ID capture impossible"
  - "Lock discipline: Arc<Mutex<HashMap>> held only for insert/remove, never across .await points"
  - "When no non-agent services present, polling loop exits vacuously; fallback separate ps call captures agent container ID"
patterns_established:
  - "Compose exec delegation pattern: ComposeProvider.exec/exec_streaming delegate directly to self.docker (DockerProvider)"
  - "Fault-tolerant teardown pattern: compose down errors logged via warn! but not propagated (D023/D038)"
  - "Compose integration test pattern: compose_provider_or_skip() + pre_clean_containers() + assert_no_containers_for_job() — mirrors docker_lifecycle.rs style"
  - "Agent keep-alive pattern: compose YAML always sets command: [sleep, 3600] on smelt-agent service"
observability_surfaces:
  - "tracing::info! at provision start, compose up -d completion, each healthcheck poll attempt (with counter), provision complete with agent container ID, teardown start/finish"
  - "eprintln!(\"Waiting for {service} to be healthy...\") to stderr for each non-agent service before polling loop starts"
  - "tracing::warn! on teardown errors (compose down non-zero or spawn failure) — never propagated"
  - "Failure shapes: SmeltError::Provider { operation: \"provision\", message: \"timed out waiting for services to become healthy after 120s\" }; SmeltError::Provider { operation: \"provision\", message: \"service {name} became unhealthy\" }"
  - "docker compose -f <path> -p <name> ps — shows live service health state during provision"
  - "docker ps --filter label=smelt.job=<name> — confirms container cleanup after teardown (used in integration test assertions)"
drill_down_paths:
  - .kata/milestones/M004/slices/S03/tasks/T01-SUMMARY.md
  - .kata/milestones/M004/slices/S03/tasks/T02-SUMMARY.md
duration: ~1h10m (T01: 25min, T02: 45min)
verification_result: passed
completed_at: 2026-03-22T16:00:00Z
---

# S03: ComposeProvider Lifecycle

**Full `RuntimeProvider` impl for `ComposeProvider`: compose up + NDJSON healthcheck polling + bollard-delegated exec + fault-tolerant teardown; proven by 3 integration tests against real Docker including Postgres healthcheck wait.**

## What Happened

**T01** replaced the empty `ComposeProvider {}` stub from S02 with the complete `RuntimeProvider` implementation. The key design choices:

- `ComposeProjectState` privately holds `project_name`, `compose_file_path`, and `_temp_dir: TempDir`. The TempDir field intentionally anchors the temp directory lifetime to the state entry — it's only dropped on teardown, after `compose down` has read the file.
- `ComposeProvider` fields: `docker: DockerProvider` (for bollard exec delegation) and `state: Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>`. The mutex is held only for HashMap operations, never across `.await` points.
- `provision()` resolves credentials, generates the compose YAML via `generate_compose_file()`, writes to a `TempDir`, runs `docker compose up -d`, then polls `docker compose ps --format json` (NDJSON) up to 60 times at 2s intervals. Readiness logic: `Health == "" && State == "running"` (no healthcheck defined) or `Health == "healthy"`. Unhealthy is an immediate error; starting waits. Agent container ID is extracted from the ps NDJSON line where `Service == "smelt-agent"`. When no non-agent services exist, the loop exits vacuously with a fallback ps call to capture the agent ID.
- Cargo changes: `tempfile` promoted from `[dev-dependencies]` to `[dependencies]` (TempDir in production struct); `serde_json` changed from `{ version = "1", optional = true }` to `"1"` unconditional (NDJSON parsing is production code, not forge-only).

**T02** wrote `crates/smelt-cli/tests/compose_lifecycle.rs` with three integration tests and discovered two bugs in the T01 implementation:

**Bug 1 (D083):** `alpine:3` exits immediately without an explicit command. After the container exits, it disappears from `docker compose ps --format json` output (only running containers appear), making agent container ID capture impossible. Fix: add `command: [sleep, "3600"]` to the smelt-agent service block in `generate_compose_file`. This matches `DockerProvider`'s behaviour.

**Bug 2 (D082):** The custom named network (`smelt-<project>`) isolated the agent from user services. User services without explicit network config are placed on Docker Compose's automatic default project network; the agent was on a separate custom network with no shared DNS. Fix: remove the custom `networks:` key from smelt-agent and the top-level `networks:` section entirely. All services now share Docker Compose's default project network, giving automatic DNS name resolution. Both fixes required updating 6 snapshot tests in `smelt-core/src/compose.rs`.

## Verification

```
# smelt-core unit tests (138) — all pass
cargo test -p smelt-core --lib
→ test result: ok. 138 passed; 0 failed

# compose_lifecycle integration tests (3) — all pass with Docker
cargo test -p smelt-cli --test compose_lifecycle
→ test result: ok. 3 passed; 0 failed (finished in ~15s)

# Full workspace — zero regressions
cargo test --workspace
→ 9 test suites, all ok, 0 FAILED
```

Integration tests confirmed:
- `test_compose_provision_exec_teardown`: provision alpine:3 agent, exec `echo hello`, assert exit 0 + stdout="hello", teardown, verify no containers with `smelt.job=compose-test-basic` label remain
- `test_compose_healthcheck_wait_postgres`: provision Postgres 16-alpine sidecar with pg_isready healthcheck, provision returns without timeout proving healthcheck wait works, exec `nc -z postgres 5432 && echo ok` from agent asserting exit 0 + stdout contains "ok" (confirms default-network DNS reachability)
- `test_compose_teardown_after_exec_error`: provision, exec `exit 1`, assert exit_code==1, teardown without error, verify no containers remain

## Requirements Advanced

- R020 (Docker Compose runtime for multi-service environments) — `ComposeProvider: RuntimeProvider` is fully implemented with provision/exec/teardown; integration tests with real Docker prove the lifecycle works; Postgres healthcheck wait proves the `docker compose ps` stability risk from the roadmap is retired

## Requirements Validated

- R020 — primary proof delivered by this slice: `ComposeProvider::new()` + `provision()` + `exec()` + `teardown()` implementation passing 3 integration tests with real Docker daemon. S04 (CLI dispatch) remains before the end-to-end `smelt run examples/job-manifest-compose.toml` flow is testable, but the infrastructure layer is proven.

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

**Two bugs discovered during integration testing** (both correctness fixes, not scope changes):

1. **Missing keep-alive command (D083)**: Plan assumed alpine:3 would remain running; it exits immediately without a command. Fix: `command: [sleep, "3600"]` added to smelt-agent in `generate_compose_file`. Required updating 6 snapshot tests.

2. **Custom network isolated agent from user services (D082)**: Plan assumed a custom named network would work for inter-service DNS. It doesn't — user services go on Docker Compose's default project network; the custom network created a separate isolated network. Fix: remove custom network entirely; rely on Docker Compose default. Required updating 6 snapshot tests.

The fallback separate ps call after the healthcheck polling loop (to handle the case where the agent container ID wasn't seen during a vacuous poll) was a minor correctness improvement over the plan, not a regression.

## Known Limitations

- `collect()` is a no-op — artifact collection is host-side in the Docker runtime model; this is consistent with `DockerProvider.collect()` and expected.
- The Postgres test requires `nc` (netcat) in the agent image. `alpine:3` includes `nc` by default; other agent images may not.
- `docker compose ps --format json` produces NDJSON (one JSON object per line). If a future Docker Compose version changes this to a JSON array, the parsing loop will need updating — but this matches current Compose v2 behaviour.

## Follow-ups

- S04: wire `ComposeProvider` into `run.rs` dispatch (`match runtime { "compose" => ComposeProvider }`), add `── Compose Services ──` section to `print_execution_plan()`, add `examples/job-manifest-compose.toml`

## Files Created/Modified

- `crates/smelt-core/src/compose.rs` — full `ComposeProvider` impl with all 5 `RuntimeProvider` methods, `ComposeProjectState` struct, updated `smoke_empty_services_compiles` test, 6 snapshot tests updated for D082/D083 fixes
- `crates/smelt-core/Cargo.toml` — `tempfile` → `[dependencies]`; `serde_json` made unconditional; `forge` feature simplified to `["dep:octocrab"]`
- `crates/smelt-cli/tests/compose_lifecycle.rs` — new integration test file (3 tests + helpers)
- `crates/smelt-cli/Cargo.toml` — added `indexmap.workspace = true` to `[dev-dependencies]`

## Forward Intelligence

### What the next slice should know

- `ComposeProvider::new()` is a failable constructor (returns `crate::Result<Self>`) because `DockerProvider::new()` can fail if the Docker daemon is unavailable. S04's `run.rs` dispatch will need to handle this.
- The compose project name is `smelt-{job_name}` (from `manifest.job.name`). Docker Compose prefixes container names with the project name — e.g. `smelt-compose-test-basic-smelt-agent-1`. The label `smelt.job=<job_name>` is the stable identifier for container cleanup.
- `generate_compose_file` no longer emits a custom `networks:` section (D082). Any documentation or dry-run output that references the network name will need to use Docker Compose's auto-generated name (`<project>_default`) rather than `smelt-<project>`.
- The snapshot tests in `smelt-core/src/compose.rs` are now authoritative — they cover the corrected YAML shape including `command: [sleep, "3600"]` on smelt-agent and no custom `networks:` key.

### What's fragile

- `docker compose ps --format json` NDJSON parsing — tested against Compose v2 on macOS/Linux. If the format changes in future Compose versions (e.g. JSON array instead of NDJSON), the healthcheck polling loop breaks silently (times out instead of erroring). The test `test_compose_healthcheck_wait_postgres` catches regressions.
- The vacuous loop exit (when no non-agent services) relies on a fallback ps call to get the agent container ID. If `compose up -d` is very slow, the agent container may not yet appear in the first fallback ps call — provision will return an error. In practice with alpine:3 this hasn't been an issue.

### Authoritative diagnostics

- `docker compose -f <path> -p <name> ps` — ground truth for service health state during provision
- `docker ps --filter label=smelt.job=<name>` — confirms container cleanup (also used in `assert_no_containers_for_job()` in integration tests)
- `tracing::info!` output at each healthcheck poll iteration includes the attempt counter — enabling timeout diagnosis

### What assumptions changed

- **Custom network for isolation**: S02 plan assumed a custom named network (`smelt-<project>`) would cleanly isolate the compose project. In practice, Docker Compose places user services without explicit network config on the default project network — the custom network was a separate isolated island with no DNS resolution to user services.
- **alpine:3 as agent image**: The plan assumed alpine:3 would remain running after `compose up -d`. It doesn't — it exits immediately without a command. The keep-alive pattern (`sleep 3600`) is required for any non-server agent image.
