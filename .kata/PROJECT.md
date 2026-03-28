# Smelt — Project Context

## What Smelt Is

Smelt is the infrastructure layer in the smelt/assay/cupel agentic development toolkit. It provisions isolated Docker environments, mounts the host repo, delegates orchestration to Assay inside the container, streams gate output to the terminal, collects the result branch, and creates a GitHub PR for human review. The output of a `smelt run` is a pull request, ready for review.

A user who wants containerized, isolated, forge-integrated AI coding sessions runs `smelt run` — same orchestration logic as raw `assay run`, with infrastructure provisioning and PR delivery wrapped around it.

## Core Value

Automated infrastructure delivery: `smelt run manifest.toml` provisions → runs Assay → creates PR. The user reviews the PR; Smelt owns everything before that.

## Current State

**M011 complete.** S01 decomposed `manifest.rs` (1924L) and `git/cli.rs` (1365L) into 14 focused modules under 500 lines each (max 337L); R060 validated. S03 added unauthenticated `GET /health` endpoint via `Router::merge()`; R063 validated. S02 (carried into M012/S01) migrated 50 `eprintln!` calls to structured tracing macros and fixed `test_cli_run_invalid_manifest` timeout from 10s to 30s; R061 and R062 validated. 298 workspace tests, 0 failures.

**M012 in progress (S01 ✅, S02 ✅).** S01 cleaned up M011 leftover work: three-way tracing subscriber init (D158), full eprintln! migration (D159), flaky test timeout fix. R061 and R062 now validated. S02 delivered the full tracker contract layer: TrackerSource trait (RPITIT), TrackerConfig with deny_unknown_fields, ServerConfig integration with collected validation, load_template_manifest() + issue_to_manifest() + MockTrackerSource, StateBackendConfig mirror enum, and JobManifest.state_backend field. 337 workspace tests, 0 failures. S03 next: GithubTrackerSource via `gh` CLI.

**M010 complete.** HTTP API authentication and code quality. S01 delivered bearer token auth for `smelt serve`: opt-in `[auth]` config with env var resolution, read/write permission split middleware (GET/HEAD = read, POST/DELETE = write), 401/403 JSON error responses, and 4 integration tests covering all token×permission combinations. S02 cleaned up two PR review debt items: extracted `warn_teardown()` replacing 6 silent `let _ =` blocks with logged warnings, replaced 5 `anyhow!("{e}")` with `.context()`, and extracted `build_common_ssh_args()` eliminating SSH/SCP flag duplication. S03 documented the `[auth]` section in `examples/server.toml` and README.md, then verified all milestone success criteria (290 tests, clippy clean, doc clean). R050–R053 validated.

**M009 complete.** Documentation, examples, and code cleanup. S01 enforced `#![deny(missing_docs)]` on smelt-cli, audited stale `#[allow]` annotations, fixed cargo doc warnings. S02 wrote comprehensive README.md and annotated all example manifests. S03 decomposed three large files (run.rs, ssh.rs, serve/tests.rs) into focused modules. 286 tests green.

**M008 complete.** SSH worker pools — `smelt serve` dispatches jobs to remote machines via SSH. S01 delivered `WorkerConfig` + `SshClient` trait + probe timeout. S02 added `deliver_manifest()` + `run_remote_job()` with MockSshClient. S03 added `sync_state_back()` for recursive remote-to-local state sync. S04 wired everything into `dispatch_loop`: round-robin worker selection with probe-based offline skip, all-workers-offline re-queue, `worker_host` visible in `GET /api/v1/jobs` and TUI Worker column. 286 workspace tests green (81 smelt-cli + 155 smelt-core + integration + doctests), 0 failures. R027 validated. Live multi-host proof deferred to S04-UAT.md.

**M007 complete.** `smelt serve` now survives restarts without losing queued work. Queue state is written atomically to `queue_dir/.smelt-queue-state.toml` on every enqueue/complete/cancel (S02) and loaded on startup via `ServerState::load_or_new()` (S03). Jobs that were Queued/Retrying/Running at crash time are automatically re-dispatched on restart with attempt counts preserved. R028 (persistent queue) validated. 52 smelt-cli tests pass.

**M006 complete.** `smelt serve --config server.toml` is the primary new capability. A long-running daemon accepts job manifests via directory watch (drop a `.toml` into `queue_dir/`) or HTTP POST (`/api/v1/jobs`), dispatches up to `max_concurrent` concurrent `smelt run` sessions, auto-retries failures, and displays a live Ratatui TUI table of all jobs. Ctrl+C broadcasts cancellation to all running job tasks via `CancellationToken`. R023 (parallel dispatch), R024 (HTTP API), and R025 (live TUI) are all validated. `cargo test --workspace` green. Live TUI rendering + Ctrl+C teardown with real Docker containers deferred to S03-UAT.md.

**M002 complete.** Smelt integrates a real Assay binary with contract-correct manifest generation, streaming output, and exit-code semantics:

- `smelt run manifest.toml` provisions a container, writes `.assay/` setup (config + per-session spec files), runs `assay run` with the correct `[[sessions]]`-keyed manifest, streams gate output to the terminal as it arrives, collects the result branch, and tears down
- `smelt run --dry-run` validates the manifest and prints the execution plan without touching Docker
- `smelt status` shows live job progress (phase, container ID, sessions, elapsed time)
- Exit code 2 from `assay run` (gate failures) is surfaced as `JobPhase::GatesFailed` — `smelt run` exits 2, not 1
- Container lifecycle is robust: timeout enforcement, Ctrl+C handling, and idempotent teardown
- 23 Docker integration tests including real-assay binary parsing proof and streaming chunk delivery

**M003 complete (pending human UAT).** All six slices shipped. S01 delivered the `smelt_core::forge` module: `ForgeClient` trait, `GitHubForge` impl backed by octocrab, all forge types (`PrHandle`, `PrStatus`, `PrState`, `CiStatus`, `ForgeConfig`) — unit-tested with wiremock mock HTTP servers. S02 wired `GitHubForge::create_pr()` into `execute_run()` Phase 9: `JobManifest` accepts optional `[forge]` section, `RunState` persists `pr_url`/`pr_number`, `smelt run --no-pr` skips PR creation. S03 added `smelt status` PR section and `smelt watch <job-name>` blocking poll command (exits 0 on Merged, 1 on Closed; MockForge-testable). S04 delivered infrastructure hardening: per-job state isolation, `smelt init`, `smelt list`, `.assay/` gitignore guard. S05 polished `smelt-core` as a library: `#![deny(missing_docs)]`, Cargo metadata, external crate embedding proof via `/tmp/smelt-example`. S06 closed out with zero cargo doc warnings, 30 stale issues archived, DRY/fragility fixes in git/cli.rs, and the human UAT script. All 14 active requirements are now validated. Live proof (real Docker + real GITHUB_TOKEN) awaits human execution of S06-UAT.md.

**M004 complete.** Docker Compose runtime for multi-service environments. S01 extended `JobManifest` with `ComposeService` struct and `[[services]]` array with full passthrough via `IndexMap<String, toml::Value>`. S02 delivered `generate_compose_file()` with smelt-agent injection, default project network (D082), `command: [sleep, "3600"]` for agent keep-alive (D083), and snapshot tests proving TOML→YAML type fidelity. S03 implemented `ComposeProvider: RuntimeProvider` with `tempfile::TempDir`-backed compose project management, healthcheck polling via `docker compose ps --format json`, and three integration tests against real Docker (provision + exec + teardown, healthcheck wait with real Postgres, teardown after error). S04 wired everything: `enum AnyProvider { Docker, Compose }` in `run.rs` dispatches by `manifest.environment.runtime`; `--dry-run` shows `── Compose Services ──` section; `examples/job-manifest-compose.toml` ships as the canonical compose example. R020 validated across all four slices. 220 workspace tests, 0 failures.

**M006 complete.** `smelt serve --config server.toml` is a working parallel dispatch daemon. All three slices shipped: S01 (JobQueue + in-process dispatch, CancellationToken broadcast teardown), S02 (DirectoryWatcher + axum HTTP API: POST/GET/DELETE /api/v1/jobs), S03 (final assembly: ServerConfig TOML parsing, smelt serve subcommand wiring dispatch_loop + DirectoryWatcher + axum + Ratatui TUI under tokio::select!, Ctrl+C graceful shutdown, tracing redirect to .smelt/serve.log in TUI mode, examples/server.toml). R023, R024, and R025 are validated. `cargo test --workspace` green (46 smelt-cli + 155 smelt-core + all integration tests). Live TUI rendering + Ctrl+C teardown with real Docker containers deferred to S03-UAT.md.

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
  smelt-cli/    — smelt binary: run, status, watch (M003), init (M003), serve (M006) subcommands
examples/
  job-manifest.toml          — valid example manifest
  job-manifest-compose.toml  — compose runtime example (M004)
  job-manifest-k8s.toml      — kubernetes runtime example (M005)
  server.toml                — smelt serve daemon config (M006)
  bad-manifest.toml          — invalid manifest for testing
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
| M006 | Parallel Dispatch Daemon | ✅ Complete (2026-03-23, pending live UAT) |
| M007 | Persistent Queue | ✅ Complete (2026-03-23) |
| M008 | SSH Worker Pools | ✅ Complete (2026-03-24, pending live UAT via S04-UAT.md) |
| M009 | Documentation, Examples & Code Cleanup | ✅ Complete (2026-03-24) |
| M010 | HTTP API Authentication & Code Quality | ✅ Complete (2026-03-24) |
| M011 | Code Quality III & Operational Hardening | ✅ Complete (2026-03-27) |
| M012 | Tracker-Driven Autonomous Dispatch | 🔄 In Progress — S01 ✅, S02 ✅, S03–S05 remaining |

## Technology Decisions

| Decision | Choice | Notes |
|----------|--------|-------|
| Docker client | bollard | bollard 0.20 query params at `bollard::query_parameters::*` |
| Async traits | RPITIT (not async_trait) | Rust 2024 edition; makes trait not object-safe — use generics not dyn |
| Manifest parsing | deny_unknown_fields on all structs | Strict schema enforcement |
| Container keep-alive | sleep 3600 CMD, work via exec | Container stays running between exec calls |
| Cancellation | Generic future for single-job; CancellationToken (tokio-util) for multi-job serve | D037/D099; child token per job for broadcast |
| State file | .smelt/runs/<job>/state.toml TOML | Per-job isolation (M003/S04); backward-compat fallback reads flat .smelt/run-state.toml |
| Manifest delivery | Base64-encode + exec base64 -d | Avoids heredoc quoting issues |
| Result collection | Host-side via GitOps | Bind-mount means commits already on host |
| GitHub API | octocrab crate (D052) | Async, tokio-native; host-side only; forge tokens never enter container |
