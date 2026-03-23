# Smelt — Project Context

## What Smelt Is

Smelt is the infrastructure layer in the smelt/assay/cupel agentic development toolkit. It provisions isolated Docker environments, mounts the host repo, delegates orchestration to Assay inside the container, streams gate output to the terminal, collects the result branch, and creates a GitHub PR for human review. The output of a `smelt run` is a pull request, ready for review.

A user who wants containerized, isolated, forge-integrated AI coding sessions runs `smelt run` — same orchestration logic as raw `assay run`, with infrastructure provisioning and PR delivery wrapped around it.

## Core Value

Automated infrastructure delivery: `smelt run manifest.toml` provisions → runs Assay → creates PR. The user reviews the PR; Smelt owns everything before that.

## Current State

**M002 complete.** Smelt integrates a real Assay binary with contract-correct manifest generation, streaming output, and exit-code semantics:

- `smelt run manifest.toml` provisions a container, writes `.assay/` setup (config + per-session spec files), runs `assay run` with the correct `[[sessions]]`-keyed manifest, streams gate output to the terminal as it arrives, collects the result branch, and tears down
- `smelt run --dry-run` validates the manifest and prints the execution plan without touching Docker
- `smelt status` shows live job progress (phase, container ID, sessions, elapsed time)
- Exit code 2 from `assay run` (gate failures) is surfaced as `JobPhase::GatesFailed` — `smelt run` exits 2, not 1
- Container lifecycle is robust: timeout enforcement, Ctrl+C handling, and idempotent teardown
- 23 Docker integration tests including real-assay binary parsing proof and streaming chunk delivery

**M003 complete (pending human UAT).** All six slices shipped. S01 delivered the `smelt_core::forge` module: `ForgeClient` trait, `GitHubForge` impl backed by octocrab, all forge types (`PrHandle`, `PrStatus`, `PrState`, `CiStatus`, `ForgeConfig`) — unit-tested with wiremock mock HTTP servers. S02 wired `GitHubForge::create_pr()` into `execute_run()` Phase 9: `JobManifest` accepts optional `[forge]` section, `RunState` persists `pr_url`/`pr_number`, `smelt run --no-pr` skips PR creation. S03 added `smelt status` PR section and `smelt watch <job-name>` blocking poll command (exits 0 on Merged, 1 on Closed; MockForge-testable). S04 delivered infrastructure hardening: per-job state isolation, `smelt init`, `smelt list`, `.assay/` gitignore guard. S05 polished `smelt-core` as a library: `#![deny(missing_docs)]`, Cargo metadata, external crate embedding proof via `/tmp/smelt-example`. S06 closed out with zero cargo doc warnings, 30 stale issues archived, DRY/fragility fixes in git/cli.rs, and the human UAT script. All 14 active requirements are now validated. Live proof (real Docker + real GITHUB_TOKEN) awaits human execution of S06-UAT.md.

**M004 complete.** Docker Compose runtime for multi-service environments. S01 extended `JobManifest` with `ComposeService` struct and `[[services]]` array with full passthrough via `IndexMap<String, toml::Value>`. S02 delivered `generate_compose_file()` with smelt-agent injection, default project network (D082), `command: [sleep, "3600"]` for agent keep-alive (D083), and snapshot tests proving TOML→YAML type fidelity. S03 implemented `ComposeProvider: RuntimeProvider` with `tempfile::TempDir`-backed compose project management, healthcheck polling via `docker compose ps --format json`, and three integration tests against real Docker (provision + exec + teardown, healthcheck wait with real Postgres, teardown after error). S04 wired everything: `enum AnyProvider { Docker, Compose }` in `run.rs` dispatches by `manifest.environment.runtime`; `--dry-run` shows `── Compose Services ──` section; `examples/job-manifest-compose.toml` ships as the canonical compose example. R020 validated across all four slices. 220 workspace tests, 0 failures.

**M005 complete.** Kubernetes runtime — `KubernetesProvider: RuntimeProvider` enables Assay sessions on any K8s cluster. S01 delivered the manifest foundation: `KubernetesConfig`, `generate_pod_spec()`, `KubernetesProvider` stub, `examples/job-manifest-k8s.toml`, kube/k8s-openapi deps. S02 delivered the full provider implementation: `KubernetesProvider::new()` (context-aware kubeconfig), `provision()` (SSH Secret creation + Pod creation + 60×2s readiness polling), `exec()` (buffered WebSocket attach), `exec_streaming()` (sequential FnMut callback), `teardown()` (idempotent Pod+Secret deletion). S03 delivered the push-from-Pod collection path: `SMELT_GIT_REMOTE` env var injected into agent container via `generate_pod_spec()`, `GitOps::fetch_ref()` + `GitCli` implementation (force-refspec, bare repo unit test), Phase 8 kubernetes fetch block in `run.rs` calling `fetch_ref("origin", "+<target>:<target>")` before `ResultCollector`. S04 wired CLI dispatch: `AnyProvider::Kubernetes(KubernetesProvider)` in `run.rs` with 5 delegation arms, Phase 3 async dispatch, `── Kubernetes ──` dry-run section in `print_execution_plan()`, and `dry_run_kubernetes_manifest_shows_kubernetes_section` integration test. All 27 dry-run tests green, 155+ workspace unit tests, 0 failures. R021 validated. Live end-to-end proof (real kind cluster + real Assay image) deferred to S04-UAT.md.

## Architecture

- **Role:** Pure infrastructure layer — Smelt provisions environments, Assay owns orchestration (D001)
- **Assay integration:** Shell out to `assay` CLI; no crate dependency (D002)
- **Runtime abstraction:** Pluggable `RuntimeProvider` trait — Docker first, Compose/K8s via same trait (D004)
- **Repo delivery:** Bind-mount host repo into container at `/workspace` (D013)
- **Credential injection:** Environment variable passthrough (D014); forge tokens never enter container
- **Manifest authorship:** Assay generates manifests, Smelt consumes (D010)
- **Forge integration:** GitHub API via `octocrab`; host-side only (D052)

## Workspace Structure

```
crates/
  smelt-core/   — manifest types, RuntimeProvider trait, DockerProvider, AssayInvoker,
                  ResultCollector, JobMonitor, GitOps, SmeltConfig, SmeltError,
                  ForgeClient (M003), GitHubForge (M003)
  smelt-cli/    — smelt binary: run, status, watch (M003), init (M003) subcommands
examples/
  job-manifest.toml   — valid example manifest (will gain [forge] section in M003)
  bad-manifest.toml   — invalid manifest for testing
```

## Ecosystem

| Layer | Project | Responsibility |
|-------|---------|---------------|
| Infrastructure | **Smelt** | Container provisioning, forge delivery, environment isolation |
| Orchestration | **Assay** | Spec-driven sessions, dual-track quality gates, multi-agent coordination |
| Context | **Cupel** | Token-budgeted context window optimization (library, consumed by Assay) |

## Milestones

| Milestone | Title | Status |
|-----------|-------|--------|
| M001 | Docker-First Infrastructure MVP | ✅ Complete (2026-03-17) |
| M002 | Real Assay Integration | ✅ Complete (2026-03-17) |
| M003 | Forge-Integrated Infrastructure Platform | ✅ Complete (2026-03-21, pending live UAT) |
| M004 | Docker Compose Runtime | ✅ Complete (2026-03-23) |
| M005 | Kubernetes Runtime | ✅ Complete (2026-03-23, pending live UAT) |
| M006 | Parallel Dispatch Daemon | 🔄 In progress (S02 done, S03 remaining) |

## Technology Decisions

| Decision | Choice | Notes |
|----------|--------|-------|
| Docker client | bollard | bollard 0.20 query params at `bollard::query_parameters::*` |
| Async traits | RPITIT (not async_trait) | Rust 2024 edition; makes trait not object-safe — use generics not dyn |
| Manifest parsing | deny_unknown_fields on all structs | Strict schema enforcement |
| Container keep-alive | sleep 3600 CMD, work via exec | Container stays running between exec calls |
| Cancellation | Generic future (not CancellationToken) | oneshot in tests, ctrl_c() in prod |
| State file | .smelt/runs/<job>/state.toml TOML | Per-job isolation (M003/S04); backward-compat fallback reads flat .smelt/run-state.toml |
| Manifest delivery | Base64-encode + exec base64 -d | Avoids heredoc quoting issues |
| Result collection | Host-side via GitOps | Bind-mount means commits already on host |
| GitHub API | octocrab crate (D052) | Async, tokio-native; host-side only; forge tokens never enter container |
