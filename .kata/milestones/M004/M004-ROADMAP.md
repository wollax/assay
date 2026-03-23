# M004: Docker Compose Runtime

**Vision:** Smelt provisions multi-container environments — Assay's agent runs alongside real service dependencies (Postgres, Redis, etc.) in a Docker Compose stack generated at runtime from the job manifest, with full passthrough of any Compose service field.

## Success Criteria

- `smelt run manifest.toml` with `runtime = "compose"` and `[[services]]` entries provisions a Compose stack, waits for all services to be healthy, runs Assay in the agent container, and tears down completely — `docker ps` shows nothing after completion
- Service containers are reachable by name from the agent container (e.g., `postgres:5432` is accessible inside `smelt-agent`)
- `smelt run --dry-run` with a compose manifest shows a `── Compose Services ──` section and exits 0 without touching Docker
- Ctrl+C during `smelt run` tears down the full Compose stack cleanly via `docker compose down`
- Any Docker Compose service field (image, environment, healthcheck, ports, volumes, depends_on, etc.) passes through from `[[services]]` to the generated compose file without modification
- `smelt run` without `[[services]]` (i.e., `runtime = "docker"`) is completely unchanged

## Key Risks / Unknowns

- **TOML → YAML type fidelity** — `[[services]]` entries are parsed as `HashMap<String, toml::Value>` and serialized to YAML via `serde_yaml`. Integer, boolean, and array values must preserve types correctly (e.g., `test = ["CMD", "pg_isready"]` must become a YAML sequence, not a string).
- **`docker compose ps --format json` stability** — The JSON output format for healthcheck polling may vary between Docker Compose versions or differ for services without an explicit healthcheck. The polling loop must handle all states gracefully.
- **ComposeProvider internal state** — `teardown(container_id)` receives an opaque `ContainerId` but must know the compose project name and temp file path. The provider needs an internal `HashMap<ContainerId, ComposeProjectState>` to track this across the lifecycle.

## Proof Strategy

- **TOML → YAML type fidelity** → retire in S02 by snapshot-testing `generate_compose_file()` with known compose service definitions (healthcheck array, integer timeout, boolean restart) and asserting exact YAML output.
- **`docker compose ps` stability** → retire in S03 by running a real healthcheck wait against a live Postgres container and asserting the agent starts only after `pg_isready` passes.
- **ComposeProvider internal state** → retire in S03 by design: `Arc<Mutex<HashMap<ContainerId, ComposeProjectState>>>` on the provider struct; tests confirm teardown finds the right project.

## Verification Classes

- Contract verification: `[[services]]` roundtrip tests; `generate_compose_file()` snapshot tests for smelt-agent injection, network, credentials, and service passthrough; validation tests for required fields and `runtime` guard.
- Integration verification: `smelt run examples/job-manifest-compose.toml` provisions a real Postgres + agent stack; agent reaches Postgres; both containers removed after teardown.
- Operational verification: Ctrl+C teardown kills the full Compose stack; `docker ps` shows nothing after completion; `--dry-run` works without touching Docker.
- UAT / human verification: none beyond the integration tests — all acceptance criteria are machine-verifiable.

## Milestone Definition of Done

This milestone is complete only when all are true:

- `ComposeService` type with `name`, `image`, and `HashMap<String, toml::Value>` passthrough is in `JobManifest`; roundtrip and validation tests pass
- `generate_compose_file()` produces valid YAML with smelt-agent injection; snapshot tests confirm YAML structure for a Postgres + Redis service definition
- `ComposeProvider: RuntimeProvider` impl passes integration tests with real Docker: provision (compose up + healthcheck wait) → exec/exec_streaming (bollard on agent container) → teardown (compose down)
- `smelt run examples/job-manifest-compose.toml` provisions the stack, runs Assay in the agent, tears down — confirmed by `docker ps` before and after
- `smelt run examples/job-manifest-compose.toml --dry-run` exits 0 and prints `── Compose Services ──` section
- Existing `runtime = "docker"` tests are unaffected — zero regressions in the workspace test suite

## Requirement Coverage

- Covers: R020
- Partially covers: none
- Leaves for later: R021 (multi-machine), R022 (budget tracking)
- Orphan risks: none

## Slices

- [x] **S01: Manifest Extension** `risk:low` `depends:[]`
  > After this: `cargo test -p smelt-core` proves `[[services]]` roundtrip and validation; `smelt run --dry-run` parses a compose manifest without errors; `cargo test --workspace` shows zero regressions.

- [x] **S02: Compose File Generation** `risk:medium` `depends:[S01]`
  > After this: `generate_compose_file()` produces valid YAML for any `[[services]]` definition with smelt-agent injection; snapshot tests cover Postgres + Redis + empty-services cases; `serde_yaml` type fidelity proven for arrays, integers, and booleans.

- [x] **S03: ComposeProvider Lifecycle** `risk:high` `depends:[S01,S02]`
  > After this: `ComposeProvider: RuntimeProvider` is unit-tested for the full lifecycle; `smelt run examples/job-manifest-compose.toml` provisions a real Postgres stack, runs a test command in the agent, and tears down — `docker ps` confirms no containers remain.

- [x] **S04: CLI Integration + Dry-Run** `risk:low` `depends:[S01,S02,S03]`
  > After this: `smelt run` dispatches to `ComposeProvider` on `runtime = "compose"`; `--dry-run` shows compose services; `examples/job-manifest-compose.toml` is a working example; `smelt run manifest.toml` for `runtime = "docker"` is provably unchanged.

## Boundary Map

### S01 → S02, S03, S04

Produces:
- `ComposeService` struct: `name: String`, `image: String`, `#[serde(flatten)] extra: IndexMap<String, toml::Value>` (or similar passthrough)
- `JobManifest.services: Vec<ComposeService>` — parsed from `[[services]]` TOML array; empty by default
- `Environment.runtime` validation: allowed values are `"docker"` and `"compose"`; if `"compose"` with non-empty `services`, all entries validated for `name` and `image`; if not `"compose"` and `services` is non-empty, validation error
- Existing `environment.image` validation unchanged — required for both runtimes (agent container image)
- `JobManifest::validate()` extended with compose-specific checks
- Unit tests: manifest roundtrip with `[[services]]` present and absent; validation errors for missing `name`, missing `image`, and `runtime = "docker"` with services

Consumes:
- nothing (independent)

### S02 → S03, S04

Produces:
- `ComposeProvider` struct (in `smelt-core::compose` module, behind no feature flag)
- `generate_compose_file(manifest: &JobManifest, project_name: &str, extra_env: &HashMap<String, String>) -> String` — public function producing valid YAML
- Generated YAML structure:
  - `services:` section containing all `[[services]]` entries (name = key, extra fields merged in)
  - `smelt-agent:` service with: `image` from `environment.image`, `volumes: [<repo_path>:/workspace]`, `environment:` map from credentials env, `depends_on:` all other service names, `networks: [smelt-<project_name>]`
  - `networks:` section defining `smelt-<project_name>`
- `serde_yaml` added as production dep in `smelt-core/Cargo.toml`
- Snapshot tests: Postgres-only, Postgres + Redis, empty services (agent only); type fidelity for array healthcheck, integer timeout, boolean values

Consumes from S01:
- `ComposeService` type, `JobManifest.services`, `environment.image`

### S03 → S04

Produces:
- `ComposeProvider: RuntimeProvider` impl:
  - `provision(manifest) -> ContainerId`: writes compose file to `tempfile::TempDir`, runs `docker compose -f <path> up -d`, polls `docker compose -f <path> ps --format json` until all non-agent services report healthy/running, stores `ComposeProjectState { project_name, compose_file_path, temp_dir }` keyed by ContainerId, returns agent ContainerId
  - `exec(container, command) -> ExecHandle`: delegates to internal `DockerProvider` bollard call on the agent container ID
  - `exec_streaming(container, command, cb) -> ExecHandle`: same delegation pattern
  - `teardown(container) -> ()`: looks up `ComposeProjectState`, runs `docker compose -f <path> down --remove-orphans`, removes temp dir
- `ComposeProjectState { project_name: String, compose_file_path: PathBuf, _temp_dir: TempDir }` — private internal type
- Integration tests (skip if no Docker): provision + exec echo + teardown; healthcheck wait with real Postgres; teardown after exec error
- Stderr progress lines: `Waiting for <service> to be healthy...` (printed before healthcheck polling starts)

Consumes from S01:
- `JobManifest.services`, `environment.image`, `environment.runtime`

Consumes from S02:
- `generate_compose_file()`, `ComposeProvider` struct

### S04 (final wiring — no new public surfaces)

Produces:
- `run.rs` dispatch: `match manifest.environment.runtime.as_str() { "docker" => DockerProvider, "compose" => ComposeProvider, _ => error }`
- `print_execution_plan()` extension: `── Compose Services ──` section listing each `[[services]]` entry name + image when `runtime = "compose"`
- `examples/job-manifest-compose.toml` with a Postgres 16 service, healthcheck, and smelt-agent image
- `cargo test --workspace` all green; existing `runtime = "docker"` integration tests unaffected

Consumes from S01, S02, S03:
- `JobManifest.services`, `ComposeProvider: RuntimeProvider`
