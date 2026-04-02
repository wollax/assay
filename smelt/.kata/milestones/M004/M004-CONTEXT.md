# M004: Docker Compose Runtime — Context

**Gathered:** 2026-03-21
**Status:** Ready for planning

## Project Description

Smelt is the infrastructure layer in the smelt/assay/cupel agentic development toolkit. M001–M003 delivered single-container provisioning, real Assay integration, GitHub PR creation/tracking, and a stable `smelt-core` library API. M004 extends the runtime to multi-container environments via Docker Compose.

## Why This Milestone

Single-container provisioning (`runtime = "docker"`) is insufficient for real-world projects that depend on external services. A Rust API project needs Postgres; a Node app needs Redis; a data pipeline needs a message broker. Without a Compose runtime, Smelt can only target self-contained workloads — a significant constraint on what Assay-driven coding sessions can work on.

M004 unblocks this by adding `runtime = "compose"` as a first-class option. Assay generates a manifest that declares the services it needs, Smelt provisions the full stack, and the agent runs inside it with services reachable by name on a shared network.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Add `runtime = "compose"` to `[environment]` and declare `[[services]]` entries in a manifest to provision a full Docker Compose stack alongside the Assay agent container
- Reference compose services by name from the agent container (e.g., connect to `postgres:5432` from inside the agent)
- Have `smelt run` wait for all services to pass their healthchecks before starting Assay
- Run `smelt run manifest.toml --dry-run` with a compose manifest and see the `── Compose Services ──` section in the execution plan
- Pass any Docker Compose service field (image, environment, healthcheck, ports, volumes, depends_on, etc.) in a `[[services]]` entry — full passthrough

### Entry point / environment

- Entry point: `smelt run <compose-manifest.toml>` CLI
- Environment: local dev with Docker daemon running; Docker Compose v2 (`docker compose`) available
- Live dependencies: Docker daemon, `docker compose` CLI (v2), Assay binary in the agent container image

## Completion Class

- Contract complete means: manifest roundtrip tests for `[[services]]` entries; `generate_compose_file()` snapshot tests confirm correct YAML output including smelt-agent injection; validation tests for required fields and `runtime = "compose"` guard
- Integration complete means: `smelt run compose-manifest.toml` with a real Postgres service provisions both containers, runs the agent, and tears down cleanly (`docker ps` shows nothing after completion)
- Operational complete means: `smelt run --dry-run` shows compose services; Ctrl+C tears down the full stack via `docker compose down`

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- `smelt run examples/job-manifest-compose.toml` provisions a Postgres + agent stack, the agent can reach Postgres by hostname, and both containers are removed after the run completes
- `smelt run examples/job-manifest-compose.toml --dry-run` exits 0 and shows the `── Compose Services ──` section without touching Docker
- `docker ps` shows no containers from the run after normal exit and after Ctrl+C (teardown is unconditional)

## Risks and Unknowns

- **Healthcheck polling reliability** — `docker compose ps --format json` output format may vary between Docker Compose versions; the polling loop must handle all service states (starting, healthy, unhealthy, running) correctly. Retire in S03 by testing with a real postgres healthcheck.
- **TOML → YAML type fidelity** — Compose YAML has typed values (arrays, booleans, integers) that TOML also types. The `HashMap<String, toml::Value>` → `serde_yaml::Value` conversion must preserve types correctly (especially `test: ["CMD", "pg_isready"]` arrays). Retire in S02 by snapshot-testing known Compose service definitions.
- **ComposeProvider internal state** — `teardown(container_id)` needs to know the compose project name and temp file path to run `docker compose down`. The `ContainerId` is opaque; the provider needs internal state (HashMap) to map container → project. Retire in S03 by design: store `ComposeProjectState` keyed by ContainerId.
- **Subprocess vs bollard for exec** — `exec()` and `exec_streaming()` on the agent container can reuse bollard directly (same Docker daemon). The agent container ID is a real Docker container ID. No compose involvement after provision.

## Existing Codebase

- `crates/smelt-core/src/provider.rs` — `RuntimeProvider` trait with `provision/exec/exec_streaming/collect/teardown`; `ContainerId` is an opaque `String` wrapper; the trait is already designed for multiple impls (D004)
- `crates/smelt-core/src/docker.rs` — `DockerProvider: RuntimeProvider`; contains `exec()` and `exec_streaming()` implementations that `ComposeProvider` can reuse by delegating to a wrapped `DockerProvider` instance
- `crates/smelt-core/src/manifest.rs` — `JobManifest` with `Environment { runtime: String, image: String, ... }`; `deny_unknown_fields` on all structs; validation via `validate()` collecting errors (D018)
- `crates/smelt-cli/src/commands/run.rs` — dispatches on `manifest.environment.runtime` (currently only "docker"); Phase sequence from provision → exec → collect → teardown
- `crates/smelt-core/Cargo.toml` — `serde_yaml` not yet added; `bollard` and `tokio` already present

> See `.kata/DECISIONS.md` for all architectural and pattern decisions (D001–D071). D004 specifically anticipates the Compose runtime.

## Relevant Requirements

- R020 — Docker Compose runtime for multi-service environments (primary deliverable of M004)

## Scope

### In Scope

- `runtime = "compose"` in `[environment]` triggers `ComposeProvider`
- `[[services]]` array in manifest with full Docker Compose service passthrough via `HashMap<String, toml::Value>`
- Required fields per service entry: `name` (string) and `image` (string); all other fields pass through to generated YAML
- Smelt generates `docker-compose.yml` in a temp dir; the file is not user-visible or user-editable
- `smelt-agent` service injected automatically: uses `environment.image`, bind-mounts repo at `/workspace`, receives all `[credentials]` env vars, `depends_on` all other services
- Single shared Docker network: `smelt-<job-name>`
- Healthcheck-based readiness wait: poll `docker compose ps --format json` until all non-agent services report healthy/running
- `teardown()` runs `docker compose down --remove-orphans` (removes containers + network; does not remove named volumes by default)
- `smelt run --dry-run` with `runtime = "compose"` shows `── Compose Services ──` section
- `examples/job-manifest-compose.toml` with a Postgres service
- Credentials (from `[credentials.env]`) injected into smelt-agent only — never into service containers

### Out of Scope / Non-Goals

- Named volume management (`docker compose down -v`) — services are responsible for not persisting state across runs if desired
- Docker Compose file v3 / v2 format selection — always generate the modern format (no `version:` key required for Compose v2+)
- User-provided `docker-compose.yml` input — Smelt always generates the compose file; it does not accept or augment a user-provided file
- Multi-machine / distributed compose (swarm mode) — single-machine Docker only in M004
- GitLab / Azure DevOps forge support
- crates.io publish

## Technical Constraints

- D001 is firm: Smelt is infrastructure; Compose stack management is infrastructure.
- D004 is firm: `RuntimeProvider` trait is the abstraction; `ComposeProvider` is the new impl.
- D013: bind-mount pattern unchanged — `/workspace` inside `smelt-agent` is the host repo.
- D014: credential injection pattern unchanged — env vars forwarded to agent, not services.
- D017: `deny_unknown_fields` on `JobManifest` itself, but `ComposeService` uses `HashMap` passthrough for arbitrary compose fields.
- D019: RPITIT (no `async_trait`); `ComposeProvider` must follow same pattern.
- Docker Compose v2 only (`docker compose`, not `docker-compose`). Available as confirmed: Docker Compose version v2.40.3.
- `serde_yaml` added as a production dep in `smelt-core/Cargo.toml` for compose file generation.
- Subprocess management for `docker compose` commands — not bollard. Bollard used for `exec/exec_streaming` on the agent container only.

## Integration Points

- **Docker daemon** — same bollard connection for exec/exec_streaming on the agent container; `docker compose` CLI for provision/teardown
- **`docker compose` CLI** — provisioning (`up -d`), readiness polling (`ps --format json`), teardown (`down`)
- **Assay binary** — unchanged; runs inside smelt-agent container the same way as `DockerProvider`; no compose-specific Assay changes
- **`AssayInvoker`** — unchanged; takes a `ContainerId` and `exec_streaming`; works the same regardless of whether the container is bare Docker or a Compose service

## Open Questions

- Should `smelt run compose-manifest.toml` print the service names being waited on during healthcheck polling? (e.g., `Waiting for postgres to be healthy...`) — Yes, good UX; print to stderr.
- Should `teardown()` run `docker compose down --volumes` to also remove named volumes, or leave them? — Leave volumes by default (safest); user can add `restart: no` to services if they need clean state.
- What happens if `runtime = "compose"` but `[[services]]` is empty? — Allow it (valid edge case: user wants the compose network even with no sidecars); just inject smelt-agent into an otherwise empty compose file.
