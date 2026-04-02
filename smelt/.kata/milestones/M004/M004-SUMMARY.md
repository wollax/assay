---
id: M004
provides:
  - "`ComposeService` struct with `name`, `image`, and `#[serde(flatten)] extra: IndexMap<String, toml::Value>` passthrough for all Docker Compose service fields"
  - "`JobManifest.services: Vec<ComposeService>` with backward-compatible `#[serde(default)]`"
  - "`validate()` runtime allowlist (docker/compose) and per-service name/image checks"
  - "`generate_compose_file(manifest, project_name, extra_env) -> Result<String>` producing valid Compose YAML with smelt-agent injection, default project network, and keep-alive command"
  - "`ComposeProvider: RuntimeProvider` — full lifecycle: provision (compose up + NDJSON healthcheck polling) → exec/exec_streaming (bollard delegation) → collect (no-op) → teardown (compose down --remove-orphans)"
  - "`enum AnyProvider { Docker(DockerProvider), Compose(ComposeProvider) }` dispatch in `run.rs`"
  - "`── Compose Services ──` section in `--dry-run` output when `runtime = \"compose\"`"
  - "`examples/job-manifest-compose.toml` — canonical compose manifest with Postgres 16-alpine service and healthcheck"
  - "3 integration tests against real Docker (compose_lifecycle.rs): provision+exec+teardown, Postgres healthcheck wait, teardown after exec error"
  - "220 workspace tests, 0 failures"
key_decisions:
  - "D073: ComposeService uses IndexMap<String, toml::Value> passthrough — no schema validation of arbitrary Compose keys"
  - "D072: Smelt generates compose file at runtime in TempDir; no user-provided compose file"
  - "D074: Credentials injected into smelt-agent only; service containers receive only their own [[services]] environment block"
  - "D075: runtime = \"compose\" dispatches to ComposeProvider via match in run.rs (AnyProvider enum)"
  - "D076: serde_yaml added as production dep — generate_compose_file() runs in the normal smelt run path"
  - "D079: serde_json promoted to unconditional dep — NDJSON parsing of docker compose ps output is production code"
  - "D080: tempfile promoted to production dep — TempDir lives in ComposeProjectState across provision→teardown lifecycle"
  - "D081: Healthcheck timeout 60×2s=120s fixed constant; unhealthy state is immediate error; no manifest field"
  - "D082: smelt-agent uses Docker Compose default project network — custom named network isolated agent from user services"
  - "D083: smelt-agent always has command: [sleep, '3600'] — alpine:3 exits immediately without it; exited containers vanish from docker compose ps output"
  - "D084: AnyProvider enum in run.rs for dispatch — RuntimeProvider is not object-safe (RPITIT/D019), so dyn dispatch is impossible; enum delegation is idiomatic"
patterns_established:
  - "Compose service passthrough: arbitrary TOML keys → IndexMap<String, toml::Value> → toml_to_yaml() → serde_yaml::Value — BTreeMap key ordering for deterministic YAML"
  - "Agent keep-alive: compose YAML always sets command: [sleep, 3600] on smelt-agent service (consistent with DockerProvider)"
  - "Default network pattern: no explicit networks: key — Docker Compose auto-creates default project network giving all services shared DNS"
  - "Fault-tolerant teardown: compose down errors logged via warn! but not propagated (D023/D038)"
  - "Compose integration test pattern: compose_provider_or_skip() + pre_clean_containers() + assert_no_containers_for_job() — mirrors docker_lifecycle.rs"
  - "AnyProvider enum dispatch: local enum + RuntimeProvider impl with async fn match delegation — extend by adding variants"
  - "Conditional section in print_execution_plan(): if !manifest.services.is_empty() guard — extensible for any optional manifest section"
observability_surfaces:
  - "`smelt run <manifest> --dry-run` stdout — `── Compose Services ──` section lists all services when runtime=compose; absent when runtime=docker"
  - "`eprintln!(\"Waiting for {service} to be healthy...\")` to stderr for each non-agent service before polling loop starts"
  - "`tracing::info!` at each healthcheck poll iteration with attempt counter — enables timeout diagnosis"
  - "`tracing::warn!` on teardown errors — never propagated"
  - "`docker ps --filter label=smelt.job=<name>` — confirms container cleanup; used in integration test assertions"
  - "Failure shapes: SmeltError::Provider { operation: \"provision\", message: \"timed out waiting...\" or \"service X became unhealthy\" }"
requirement_outcomes:
  - id: R020
    from_status: active
    to_status: validated
    proof: "S01: manifest roundtrip and validation tests (131 smelt-core tests pass). S02: 7 snapshot tests prove TOML→YAML type fidelity including arrays, integers, booleans. S03: 3 integration tests against real Docker — test_compose_provision_exec_teardown, test_compose_healthcheck_wait_postgres, test_compose_teardown_after_exec_error — all pass. S04: AnyProvider dispatch wired; --dry-run shows Compose Services section; 220 workspace tests 0 failures."
duration: ~2h35m (S01: 15min, S02: 35min, S03: ~1h10m, S04: 20min, M004 summary: 15min)
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
---

# M004: Docker Compose Runtime

**`ComposeProvider: RuntimeProvider` delivers full multi-container provisioning via Docker Compose — manifest extension, YAML generation with type-fidelity-proven TOML→YAML conversion, real-Docker lifecycle with healthcheck polling, and CLI dispatch — R020 validated across four slices; 220 workspace tests, 0 failures.**

## What Happened

M004 extended Smelt from single-container to multi-container provisioning. The four slices connected cleanly in sequence.

**S01 (Manifest Extension)** established the data model: `ComposeService` with `name`, `image`, and `#[serde(flatten)] extra: IndexMap<String, toml::Value>` for full Docker Compose service passthrough. The `services: Vec<ComposeService>` field was added to `JobManifest` with `#[serde(default)]` for backward compatibility. `validate()` was extended with a runtime allowlist (`"docker"` and `"compose"` only), a services-require-compose guard, and per-service name/image non-empty checks. `indexmap` v2 was promoted to a workspace dependency. 10 new tests brought the smelt-core suite to 131; zero regressions across the workspace.

**S02 (Compose File Generation)** implemented `generate_compose_file()` — a pure function producing valid Docker Compose YAML from a `JobManifest`. The function builds a `serde_yaml::Mapping`-based document: user services (image-first, BTreeMap-ordered extra fields), a `smelt-agent` service with workspace volume mount, credential env (sorted via BTreeMap), and conditional `depends_on`. A private `toml_to_yaml()` helper covers all 7 `toml::Value` variants with correct type mapping. `serde_yaml` was added as a production dependency. 7 snapshot tests proved type fidelity (arrays, integers, booleans) and structure correctness (empty services, single service, multi-service, nested healthcheck).

**S03 (ComposeProvider Lifecycle)** was the most complex slice and produced two correctness fixes discovered during integration testing. The full `RuntimeProvider` implementation: `provision()` resolves credentials, writes the compose YAML to a `TempDir`, runs `docker compose up -d`, polls `docker compose ps --format json` (NDJSON) up to 60×2s=120s waiting for all non-agent services to become healthy, then extracts the agent container ID. `exec()` and `exec_streaming()` delegate directly to an internal `DockerProvider` via bollard. `teardown()` runs `docker compose down --remove-orphans` and drops the TempDir. Internal state is tracked via `Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>` — the mutex is never held across `.await` points. Two bugs were discovered during integration testing: D083 (alpine:3 exits immediately without `command: [sleep, "3600"]`) and D082 (custom named network isolated agent from user services — removed in favour of Docker Compose's default project network). Both fixes required updating 6 snapshot tests from S02. Three integration tests against real Docker confirmed the complete lifecycle including a real Postgres healthcheck wait.

**S04 (CLI Integration + Dry-Run)** wired everything together. `print_execution_plan()` gained a conditional `── Compose Services ──` section (rendered when `manifest.services` is non-empty). `run.rs` dispatch was replaced with `enum AnyProvider { Docker(DockerProvider), Compose(ComposeProvider) }` implementing `RuntimeProvider` via RPITIT — the enum pattern is necessary because `RuntimeProvider` is not object-safe (D019/D084). Unknown runtimes return `Ok(1)` with an explicit error message. `examples/job-manifest-compose.toml` ships as the canonical compose example with a Postgres 16-alpine service and healthcheck. Two new dry-run integration tests confirmed the section appears/absent correctly.

## Cross-Slice Verification

All six success criteria from the M004 roadmap were verified:

**1. `smelt run manifest.toml` with `runtime = "compose"` provisions, waits for health, runs, tears down cleanly**
- Verified by S03 integration tests: `test_compose_provision_exec_teardown` (alpine:3 + agent), `test_compose_healthcheck_wait_postgres` (real Postgres 16-alpine with `pg_isready` healthcheck). `docker ps` shows no containers after teardown (confirmed by `assert_no_containers_for_job()` helper).
- `cargo test -p smelt-cli --test compose_lifecycle` → 3 passed, 0 failed, finished in ~13s

**2. Service containers reachable by name from the agent container**
- Verified by `test_compose_healthcheck_wait_postgres`: exec `nc -z postgres 5432 && echo ok` from agent, assert exit 0 + stdout contains "ok". DNS name resolution via Docker Compose default project network (D082).

**3. `smelt run --dry-run` with compose manifest shows `── Compose Services ──` and exits 0 without touching Docker**
- Verified directly: `cargo run --bin smelt -- run examples/job-manifest-compose.toml --dry-run` exits 0 and shows `── Compose Services ──` with `postgres  postgres:16-alpine`.
- Also verified by `dry_run_compose_manifest_shows_services_section` integration test.

**4. Ctrl+C during `smelt run` tears down the full Compose stack via `docker compose down`**
- `teardown()` is called unconditionally in both success and error paths in `run.rs` (D023/D026). The `test_compose_teardown_after_exec_error` test proves teardown runs after an exec error (exit 1) and cleans up all containers. Signal handling (Ctrl+C → `ctrl_c()` → cancellation → teardown path) is the same code path as M001/S05 — no compose-specific regression.

**5. Any Docker Compose service field passes through from `[[services]]` to generated compose file**
- Verified by S02 snapshot tests: `test_generate_compose_type_fidelity` (integer `5432`, boolean `true`, array `command`), `test_generate_compose_nested_healthcheck` (BTreeMap sub-key ordering for nested objects). `ComposeService.extra: IndexMap<String, toml::Value>` is serialized directly via `toml_to_yaml()`.

**6. `smelt run` without `[[services]]` (`runtime = "docker"`) is completely unchanged**
- Verified by: `dry_run_docker_manifest_no_services_section` integration test (no section in stdout); all 23 docker_lifecycle integration tests pass; 220 total workspace tests, 0 failures.

**Full workspace run:**
```
cargo test --workspace
→ 9 suites, 220 tests total, 0 FAILED
```

## Requirement Changes

- R020: active → validated — Full proof chain across S01 (manifest + validation), S02 (compose YAML generation + type fidelity), S03 (ComposeProvider lifecycle with real Docker: provision, exec, teardown, Postgres healthcheck wait), S04 (CLI dispatch + dry-run UX). 220 workspace tests, 0 failures.

## Forward Intelligence

### What the next milestone should know
- `ComposeProvider: RuntimeProvider` is a stable, tested impl. The `AnyProvider` enum in `run.rs` is the extension point for any future runtimes — add a variant and an arm.
- `generate_compose_file()` no longer emits a custom `networks:` section (D082). The network name is Docker Compose's auto-generated `<project>_default`. Any documentation or feature referencing the network name should use this, not `smelt-<project>`.
- `ComposeProvider::new()` is failable (returns `crate::Result<Self>`) — any new code path that constructs it must handle the error.
- The snapshot tests in `smelt-core/src/compose.rs` are authoritative for the YAML shape — they include `command: [sleep, "3600"]` on smelt-agent and no custom `networks:` key.
- R021 (multi-machine coordination) and R022 (budget tracking) remain deferred per roadmap.

### What's fragile
- `docker compose ps --format json` NDJSON parsing — tested against Compose v2.40.3 on macOS. If a future Docker Compose version changes this to a JSON array, the healthcheck polling loop times out silently instead of erroring. `test_compose_healthcheck_wait_postgres` catches regressions.
- The vacuous loop exit (when no non-agent services) falls back to a separate `ps` call to capture the agent container ID. If `compose up -d` is very slow, the agent may not yet appear — provision will return an error in this edge case.
- `workspace_vol()` in snapshot tests uses `std::fs::canonicalize(env!("CARGO_MANIFEST_DIR"))` at test runtime — if the crate moves to a different directory, expected strings self-heal because they're generated via `format!()`.

### Authoritative diagnostics
- `cargo test -p smelt-cli --test compose_lifecycle` — definitive pass/fail for ComposeProvider lifecycle
- `cargo test -p smelt-core --lib -- compose` — runs all 7 compose unit/snapshot tests with `assert_eq!` YAML diff on failure
- `smelt run <manifest> --dry-run` stdout — fastest end-to-end check; shows runtime, services, plan without Docker
- `docker ps --filter label=smelt.job=<name>` — ground truth for container cleanup verification
- `docker compose -f <path> -p <name> ps` — live service health state during provision (for manual debugging)

### What assumptions changed
- **Custom network for isolation**: S02 planned `smelt-<project>` as a named network. Integration testing (D082) showed user services go on Docker Compose's default project network; the custom network was an isolated island with no DNS to user services. Default network is now the correct approach.
- **alpine:3 as agent image**: Plan assumed alpine:3 would remain running after `compose up -d`. It exits immediately without a command (D083). The `command: [sleep, "3600"]` keep-alive is required for any non-server agent image — consistent with `DockerProvider`.
- **SmeltError::provider() arity**: S02 plan implied a one-arg constructor. Actual API is two args (`operation`, `message`). Pattern for S03 and beyond: `SmeltError::provider("provision", e.to_string())`.

## Files Created/Modified

- `Cargo.toml` — added `indexmap = { version = "2", features = ["serde"] }` to `[workspace.dependencies]`
- `crates/smelt-core/Cargo.toml` — added `indexmap.workspace = true`, `serde_yaml = "0.9"`, promoted `tempfile` and `serde_json` to production deps
- `crates/smelt-core/src/manifest.rs` — `ComposeService` struct, `services` field on `JobManifest`, three validation blocks, `VALID_COMPOSE_MANIFEST` constant, 10 new tests
- `crates/smelt-core/src/compose.rs` — new module: `ComposeProvider` (full `RuntimeProvider` impl), `ComposeProjectState`, `generate_compose_file()`, `toml_to_yaml()`, 7 snapshot tests
- `crates/smelt-core/src/lib.rs` — added `pub mod compose;` and `pub use compose::ComposeProvider;`
- `crates/smelt-cli/src/commands/run.rs` — `── Compose Services ──` section in `print_execution_plan()`; `AnyProvider` enum + `RuntimeProvider` impl; match dispatch replacing docker-only Phase 3 guard
- `crates/smelt-cli/tests/dry_run.rs` — added `dry_run_compose_manifest_shows_services_section` and `dry_run_docker_manifest_no_services_section`
- `crates/smelt-cli/tests/compose_lifecycle.rs` — new integration test file: 3 tests + helpers
- `crates/smelt-cli/Cargo.toml` — added `indexmap.workspace = true` to `[dev-dependencies]`
- `crates/smelt-cli/tests/docker_lifecycle.rs` — added `services: vec![]` to `test_manifest_with_repo()` struct literal
- `examples/job-manifest-compose.toml` — canonical compose manifest with Postgres 16-alpine service, healthcheck, and smelt-agent
