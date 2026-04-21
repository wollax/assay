# Smelt Documentation Scan

Comprehensive analysis of the smelt subsystem within the assay monorepo. Smelt is a containerized job execution engine for agentic development workflows.

---

## Technology Stack

| Category | Technology | Version | Purpose |
|----------|-----------|---------|---------|
| Language | Rust | Edition 2024 | Primary implementation language |
| Async Runtime | tokio | 1 (full features) | Async runtime for all I/O, timers, process spawning |
| Docker SDK | bollard | 0.20 | Docker daemon interaction (provision, exec, teardown) |
| Kubernetes SDK | kube | 3 | Kubernetes Pod/Secret lifecycle, exec via WebSocket |
| Kubernetes Types | k8s-openapi | 0.27 (v1_32) | Pod, Secret, Container, Volume type definitions |
| HTTP Framework | axum | 0.8 | HTTP API server for `smelt serve` |
| HTTP Client | reqwest | 0.13 | Signal delivery, Linear API GraphQL client |
| CLI Framework | clap | 4 (derive) | Command-line argument parsing |
| TUI Framework | ratatui | 0.30 | Live terminal dashboard for `smelt serve` |
| Terminal I/O | crossterm | 0.28 | Terminal event handling for TUI |
| Serialization | serde + toml + serde_json | 1 / 1 / 1 | TOML manifest parsing, JSON event payloads |
| YAML | serde_yaml | 0.9 | Docker Compose file generation |
| Error Handling | thiserror | 2 | Derive `Error` for `SmeltError` enum |
| Error Context | anyhow | 1 | CLI-level error context chain |
| Tracing | tracing + tracing-subscriber + tracing-appender | 0.1 / 0.3 / 0.2 | Structured logging with file appender for TUI mode |
| GitHub API | octocrab | 0.49 (optional, `forge` feature) | PR creation and status polling |
| Base64 | base64 | 0.22 | Encoding manifest content for container exec injection |
| UUID | uuid | 1 (v4) | Unique signal file names, job ID generation |
| Ordered Maps | indexmap | 2 (serde) | Preserve TOML field order in Compose service passthrough |
| Temp Files | tempfile | 3 | Atomic writes, temp manifest files for dispatch |
| Binary Discovery | which | 8 | Find `git`, `ssh`, `scp` binaries on PATH |
| Cancellation | tokio-util | 0.7 | `CancellationToken` for graceful shutdown broadcast |
| Stream Utilities | tokio-stream | 0.1 | `BroadcastStream` for SSE event fan-out |
| Async Stream | futures-util | 0.3 | Stream combinators (StreamExt, TryStreamExt) |
| Semver | semver | 1 | Version parsing (workspace dep) |
| Test: Assertions | assert_cmd + predicates | 2 / 3 | CLI integration test assertions |
| Test: HTTP Mock | wiremock | 0.6 | Mock HTTP server for GitHub API tests |
| Test: TLS | rustls | 0.23 (ring) | Crypto provider for octocrab test client |

### Feature Flags

- **`forge`** (smelt-core, optional): Enables `GitHubForge` and pulls in `octocrab` + `serde_json`. smelt-cli always enables this feature.

---

## Architecture Pattern

### Two-Crate Split

```
smelt-core (library)          smelt-cli (binary)
  Domain types                  CLI entry point (clap)
  Manifest parsing              Subcommand dispatch
  RuntimeProvider trait          AnyProvider enum dispatch
  DockerProvider                 serve module (daemon)
  ComposeProvider                  HTTP API (axum)
  KubernetesProvider               Queue + persistence
  ForgeClient trait                Dispatch loop
  GitHubForge                      SSH worker pools
  GitOps trait + GitCli            Directory watcher
  ResultCollector                  Tracker poller
  JobMonitor                       TUI (ratatui)
  AssayInvoker                     Event bus + SSE
  Tracker types                    Signal delivery
```

**smelt-core** is a standalone library with no CLI dependencies. It exposes the `RuntimeProvider` trait, concrete providers (Docker, Compose, Kubernetes), forge integration, git operations, manifest parsing, and job monitoring. It depends on `assay-types` for `StateBackendConfig` and signal types.

**smelt-cli** is the binary crate. It depends on `smelt-core` with the `forge` feature enabled. It adds the daemon mode (`smelt serve`) with HTTP API, queue persistence, SSH worker dispatch, TUI dashboard, tracker polling, and event ingestion/signal delivery infrastructure.

### Job Execution Model

The system uses a **provider-based container lifecycle**:

1. **Provision** -- Create and start a container (Docker/Compose/K8s)
2. **Write Manifest** -- Inject Assay run manifest + spec files via base64-encoded exec
3. **Execute** -- Run `assay run` inside the container with streaming output
4. **Collect** -- Read git state, create target branch with `ResultCollector`
5. **Forge** -- Optionally create a GitHub PR via `GitHubForge`
6. **Teardown** -- Stop and remove the container (always runs)

The `AnyProvider` enum in smelt-cli dispatches `RuntimeProvider` calls to the concrete backend selected by `manifest.environment.runtime`, avoiding `Box<dyn RuntimeProvider>` which is not object-safe due to RPITIT async methods.

### Daemon Mode (`smelt serve`)

Runs five concurrent components under `tokio::select!`:
- **dispatch_loop** -- Polls `ServerState::try_dispatch()` every 2s, spawns local or SSH job tasks
- **DirectoryWatcher** -- Watches `queue_dir/` for `.toml` manifest files
- **axum::serve** -- HTTP API on configurable host:port
- **TrackerPoller** -- Polls GitHub Issues or Linear for `smelt:ready` labels
- **Ctrl+C handler** -- Broadcasts cancellation to all in-flight jobs via `CancellationToken`

Optional TUI runs on a dedicated `std::thread` (crossterm blocking I/O), coordinated via `Arc<AtomicBool>`.

---

## API Surface

### smelt-core Public Types and Functions

**Top-level re-exports (`lib.rs`):**
- `AssayInvoker` -- Stateless translation layer: manifest/spec TOML builders, container I/O
- `BranchCollectResult`, `ResultCollector` -- Git result collection
- `ComposeProvider` -- Docker Compose runtime provider
- `SmeltConfig` -- Project-level config from `.smelt/config.toml`
- `DockerProvider` -- Docker runtime provider
- `Result<T>`, `SmeltError` -- Error types
- `GitHubForge` (feature-gated) -- GitHub PR creation/polling
- `ForgeClient`, `ForgeConfig`, `PrHandle`, `PrState`, `PrStatus`, `CiStatus` -- Forge trait and types
- `GitCli`, `GitOps`, `preflight()` -- Git operations
- `KubernetesProvider` -- Kubernetes Pod runtime provider
- `JobManifest` -- Manifest parsing and validation
- `JobMonitor`, `JobPhase`, `RunState`, `compute_job_timeout()` -- State monitoring
- `RuntimeProvider`, `ContainerId`, `ExecHandle`, `ProvisionResult`, `CollectResult` -- Provider trait and types
- `GateSummary`, `PeerUpdate`, `SignalRequest` -- Re-exported from `assay-types::signal`

**Modules:**
- `assay` -- `AssayInvoker`, `compute_smelt_event_env()`, internal serde types (`SmeltRunManifest`, `SmeltSpec`, `SmeltCriterion`)
- `collector` -- `ResultCollector<G: GitOps>`, `BranchCollectResult`
- `compose` -- `ComposeProvider`, `generate_compose_file()`
- `config` -- `SmeltConfig`
- `docker` -- `DockerProvider`, `detect_host_address()`, `parse_memory_bytes()`, `parse_cpu_nanocpus()`
- `error` -- `SmeltError` enum (10 variants), `Result<T>` alias
- `forge` -- `ForgeClient` trait, `ForgeConfig`, `PrHandle`, `PrState`, `CiStatus`, `PrStatus`, `GitHubForge`
- `git` -- `GitOps` trait (25+ methods), `GitCli`, `GitWorktreeEntry`, `parse_porcelain()`, `preflight()`
- `k8s` -- `KubernetesProvider`, `generate_pod_spec()`
- `manifest` -- `JobManifest`, `JobMeta`, `Environment`, `CredentialConfig`, `SessionDef`, `MergeConfig`, `ComposeService`, `KubernetesConfig`, `NotifyRule`, `CredentialStatus`, `ValidationErrors`, `resolve_repo_path()`
- `monitor` -- `JobMonitor`, `JobPhase` (10 variants), `RunState`, `compute_job_timeout()`
- `provider` -- `RuntimeProvider` trait, `ContainerId`, `ProvisionResult`, `ExecHandle`, `CollectResult`
- `tracker` -- `TrackerIssue`, `TrackerState` (6 variants), `StateBackendConfig` (re-export from assay-types)

### smelt-cli Public Interface

**CLI Commands (clap):**

| Command | Description | Key Args |
|---------|-------------|----------|
| `smelt init` | Generate skeleton `job-manifest.toml` | (none) |
| `smelt list` | List past runs from `.smelt/runs/` | `--dir <DIR>` |
| `smelt run <MANIFEST>` | Execute a job manifest | `--dry-run`, `--no-pr` |
| `smelt serve -c <CONFIG>` | Start job dispatch daemon | `--no-tui` |
| `smelt status [JOB_NAME]` | Show running/completed job status | `--dir <DIR>` |
| `smelt watch <JOB_NAME>` | Poll PR until merged/closed | `--interval-secs <N>` |

**HTTP API Endpoints (`smelt serve`):**

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/health` | None | Health check `{"status":"ok"}` |
| `POST` | `/api/v1/jobs` | Write | Submit job (TOML body), returns `{"job_id": "..."}` |
| `GET` | `/api/v1/jobs` | Read | List all jobs as JSON array |
| `GET` | `/api/v1/jobs/{id}` | Read | Get single job status |
| `DELETE` | `/api/v1/jobs/{id}` | Write | Cancel queued job (409 if running) |
| `POST` | `/api/v1/events` | Write | Ingest Assay event (64KB limit) |
| `GET` | `/api/v1/events` | Read | SSE stream of events (`?job=<id>` filter) |
| `POST` | `/api/v1/jobs/{id}/signals` | Write | Deliver PeerUpdate signal to session (64KB limit) |

**TUI Dashboard:**
- Split layout: job table (top) + event pane (bottom)
- Columns: Job ID, Manifest, Source, Status, Attempt, Elapsed, Worker
- Event pane shows most recent 20 events across all jobs
- Keyboard: `q` or `Ctrl+C` to quit
- Source column shows: `HTTP`, `DirWatch`, `Tracker`

---

## Domain Model

### Core Types and Relationships

```
JobManifest
  +-- JobMeta { name, repo, base_ref }
  +-- Environment { runtime, image, resources }
  +-- CredentialConfig { provider, model, env }
  +-- SessionDef[] { name, spec, harness, timeout, depends_on }
  +-- MergeConfig { strategy, order, ai_resolution, target }
  +-- ForgeConfig? { provider, repo, token_env }
  +-- KubernetesConfig? { namespace, context, ssh_key_env, cpu/mem limits }
  +-- ComposeService[] { name, image, extra (passthrough) }
  +-- StateBackendConfig? (from assay-types)
  +-- NotifyRule[] { target_job, on_session_complete }
  +-- runtime_env (computed, not serialized)
```

### Server-Side Types (smelt-cli)

```
ServerState
  +-- jobs: VecDeque<QueuedJob>
  +-- running_count, max_concurrent
  +-- queue_dir (persistence path)
  +-- round_robin_idx (SSH worker selection)
  +-- event_bus: broadcast::Sender<AssayEvent>
  +-- events: HashMap<String, EventStore>
  +-- run_ids: HashMap<String, String>
  +-- signal_urls: HashMap<String, String>

QueuedJob
  +-- id: JobId
  +-- manifest_path: PathBuf
  +-- source: JobSource { DirectoryWatch, HttpApi, Tracker }
  +-- status: JobStatus { Queued, Dispatching, Running, Retrying, Complete, Failed }
  +-- attempt: u32
  +-- queued_at, started_at: u64
  +-- worker_host: Option<String>

AssayEvent
  +-- job_id, event_id?, received_at
  +-- payload: serde_json::Value

TrackerIssue { id, title, body, source_url }
TrackerState { Ready, Queued, Running, PrCreated, Done, Failed }
```

### Runtime Provider Hierarchy

```
RuntimeProvider (trait)
  +-- DockerProvider (bollard, single container)
  +-- ComposeProvider (docker compose subprocess + bollard exec)
  +-- KubernetesProvider (kube client, Pod + SSH Secret)

AnyProvider (enum, smelt-cli only)
  +-- Docker(DockerProvider)
  +-- Compose(ComposeProvider)
  +-- Kubernetes(KubernetesProvider)
```

### Forge and Tracker

```
ForgeClient (trait)
  +-- GitHubForge (octocrab)

TrackerSource (trait, RPITIT)
  +-- GithubTrackerSource<C: GhClient>
  +-- LinearTrackerSource<C: LinearClient>

AnyTrackerSource (enum dispatcher, solves non-object-safe RPITIT)
  +-- GitHub, Linear, Mock (test-only)
```

---

## Configuration

### Project Config: `.smelt/config.toml`

Loaded by `SmeltConfig::load(project_root)`. Returns defaults if missing.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `default_image` | String | `"ubuntu:22.04"` | Default container image |
| `credential_sources` | HashMap | `{}` | Logical name to env var mapping |
| `default_resources` | HashMap | `{}` | CPU/memory defaults |
| `default_timeout` | u64 | `600` | Session timeout in seconds |

### Job Manifest: `job-manifest.toml`

Loaded by `JobManifest::load(path)`. Strict `deny_unknown_fields` on all sections.

Key sections: `[job]`, `[environment]`, `[credentials]`, `[[session]]`, `[merge]`, `[forge]` (optional), `[kubernetes]` (optional), `[[services]]` (optional), `[state_backend]` (optional), `[[notify]]` (optional).

### Server Config: `server.toml`

Loaded by `ServerConfig::load(path)` with comprehensive validation.

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `queue_dir` | PathBuf | (required) | Queue persistence directory |
| `max_concurrent` | usize | (required, >= 1) | Max parallel jobs |
| `retry_attempts` | u32 | `3` | Max retry count |
| `retry_backoff_secs` | u64 | `5` | Backoff base (reserved) |
| `ssh_timeout_secs` | u64 | `3` | SSH connection timeout |
| `server.host` | String | `"127.0.0.1"` | HTTP bind address |
| `server.port` | u16 | `8765` | HTTP port |
| `[[workers]]` | Vec | `[]` | SSH worker pool |
| `[auth]` | Optional | None | Bearer token auth |
| `[tracker]` | Optional | None | Issue tracker polling |

### Environment Variables

| Variable | Used By | Description |
|----------|---------|-------------|
| `SMELT_LOG` / `RUST_LOG` | CLI logging | Tracing filter (falls back to scoped defaults) |
| `SMELT_EVENT_HOST` | Docker host detection | Override host address for container event injection |
| `SMELT_EVENT_URL` | Container env | Computed: `http://{host}:{port}/api/v1/events` |
| `SMELT_JOB_ID` | Container env | Job identifier injected into container |
| `SMELT_WRITE_TOKEN` | Container env | Auth token for event POST (when auth configured) |
| `ANTHROPIC_API_KEY` etc. | Credential resolution | Credential env vars mapped via `credentials.env` |
| Auth token envs | `[auth]` config | `write_token_env`, `read_token_env` reference env var names |
| `LINEAR_API_KEY` | Tracker | Linear API key env var name |
| SSH key envs | `[[workers]]` | `key_env` references env var holding SSH key path |

---

## Error Handling

### SmeltError (smelt-core)

`#[non_exhaustive]` enum with 9 variants:

| Variant | Fields | Description |
|---------|--------|-------------|
| `GitNotFound` | (none) | `git` not on PATH |
| `NotAGitRepo` | (none) | Not inside a git repo |
| `GitExecution` | `operation`, `message` | Git command failed |
| `MergeConflict` | `session`, `files` | Merge conflict detected |
| `Manifest` | `field`, `message` | Parse or validation error |
| `Provider` | `operation`, `message`, `source?` | Runtime provider failure |
| `Forge` | `operation`, `message` | GitHub API failure |
| `Tracker` | `operation`, `message` | Tracker API failure |
| `Credential` | `provider`, `message` | Credential resolution failure |
| `Config` | `path`, `message` | Config load/parse failure |
| `Io` | `operation`, `path`, `source` | I/O with context |

Convenience constructors: `SmeltError::io()`, `::provider()`, `::provider_with_source()`, `::forge()`, `::forge_with_source()`, `::credential()`, `::tracker()`, `::config()`.

### CLI Error Handling

smelt-cli uses `anyhow::Result` for command-level errors, wrapping `SmeltError` via `.with_context()`. Commands return `Result<i32>` where the `i32` is the process exit code. `main()` maps `Ok(code)` to `std::process::exit(code)` and `Err(e)` to stderr + exit 1.

---

## Testing Patterns

### Unit Tests (in-module `#[cfg(test)]`)

Both crates use extensive in-module tests. Patterns observed:

- **Temp directories**: `tempfile::TempDir` for isolated filesystem tests
- **Real git repos**: `setup_test_repo()` creates temp git repos with initial commits for `GitCli` and `ResultCollector` tests
- **Mock servers**: `wiremock::MockServer` for GitHub API (forge tests)
- **Mock traits**: `MockSshClient`, `MockTrackerSource`, `MockForge` -- VecDeque-based test doubles that return pre-configured responses
- **TOML round-trip**: Serialize, deserialize, assert structural equality
- **Snapshot-style**: Exact YAML/TOML string comparison for Compose file generation
- **Table-driven**: Vec of `(input, expected)` pairs for sanitization tests
- **Compile-time assertions**: Zero-cost type-level assertions (e.g., `_assert_octocrab_error_send_sync`)

### Integration Tests

- **Docker lifecycle tests** (`docker_lifecycle`): Require running Docker daemon, marked with `#[ignore]` for `just test-smelt-unit`
- **CLI integration tests**: `assert_cmd` for binary invocation tests
- **TUI rendering tests**: `ratatui::backend::TestBackend` for pixel-level TUI verification

### Test Organization (smelt-core)

```
smelt-core/src/
  git/cli/tests/
    mod.rs (shared setup_test_repo)
    basic.rs, branch.rs, commit.rs, merge.rs, worktree.rs
  manifest/tests/
    mod.rs (shared VALID_MANIFEST constants)
    core.rs, compose.rs, forge.rs, kubernetes.rs
```

### Test Organization (smelt-cli)

```
smelt-cli/src/serve/tests/
  mod.rs
  config.rs, dispatch.rs, events.rs, http.rs, notify.rs, queue.rs, signals.rs, ssh_dispatch.rs
```

---

## Entry Points

### Binary Entry Point

`/Users/wollax/Git/personal/assay/smelt/crates/smelt-cli/src/main.rs`

- Binary name: `smelt` (configured in `Cargo.toml [[bin]]`)
- Parses `Cli` struct via `clap::Parser`
- Configures tracing based on `SMELT_LOG`/`RUST_LOG` env vars
- TUI mode redirects logs to `.smelt/serve.log`
- Dispatches to command handlers, returns exit code

### Daemon Startup (`smelt serve`)

`/Users/wollax/Git/personal/assay/smelt/crates/smelt-cli/src/commands/serve.rs`

1. Load `ServerConfig` from TOML file
2. Create `queue_dir/` if absent
3. Initialize `ServerState::load_or_new()` (restores persisted queue, remaps Running->Queued)
4. Create broadcast channel for event bus
5. Bind TCP listener on configured host:port
6. Resolve auth tokens from env vars
7. Detect Docker host address for event injection
8. Build axum router with auth middleware
9. Initialize DirectoryWatcher
10. Build TrackerPoller (if `[tracker]` configured)
11. Spawn TUI thread (if `--no-tui` not set)
12. Run all components under `tokio::select!`

### HTTP Server Bind

Binds via `TcpListener::bind(&addr)` in `serve.rs`. The actual port is logged from `listener.local_addr()` (supports port 0 for tests). The router is served via `axum::serve(listener, router)`.

---

## Key Abstractions

### `RuntimeProvider` trait

The core abstraction for container execution backends. Defines the lifecycle:
`provision() -> exec() / exec_streaming() -> collect() -> teardown()`

All methods are async and return `crate::Result`. Implementors must be `Send + Sync` for concurrent session execution. Three implementations: `DockerProvider`, `ComposeProvider`, `KubernetesProvider`.

### `GitOps` trait

25+ async methods for git operations. Production implementation: `GitCli` (shells out to `git` binary via `tokio::process::Command`). Exists as a test seam -- tests can substitute fakes. Key operations: worktree management, branch operations, merge (including squash), rev-parse, diff, fetch.

### `ForgeClient` trait

PR lifecycle operations: `create_pr()` and `poll_pr_status()`. Implementation: `GitHubForge` (octocrab, feature-gated).

### `TrackerSource` trait

RPITIT-based async trait for external issue trackers: `poll_ready_issues()` and `transition_state()`. Not object-safe. Solved via `AnyTrackerSource` enum dispatch pattern.

### `SshClient` trait

Async SSH operations: `exec()`, `probe()`, `scp_to()`, `scp_from()`. Implementation: `SubprocessSshClient` (shells out to system `ssh`/`scp`). Uses `-o ConnectTimeout` for fast-fail.

---

## Internal Data Flow

### Job Submission to Completion

```
Job Submission (3 paths)
  |
  +-- HTTP API: POST /api/v1/jobs (TOML body) -> parse + validate -> enqueue
  +-- Directory Watch: .toml file in queue_dir/ -> rename to dispatched/ -> parse + validate -> enqueue
  +-- Tracker Poller: poll ready issues -> transition Ready->Queued -> inject session -> write temp file -> enqueue
  |
  v
ServerState.enqueue(manifest_path, source) -> QueuedJob { status: Queued }
  |   (persists to .smelt-queue-state.toml)
  v
dispatch_loop (every 2s)
  |-- try_dispatch() -> promotes Queued -> Dispatching, increments running_count
  |
  +-- Local path (no workers): spawn run_job_task()
  |     |-- Inject runtime_env (SMELT_EVENT_URL, SMELT_JOB_ID, SMELT_WRITE_TOKEN)
  |     |-- Build RunArgs
  |     |-- Create container_ip oneshot channel
  |     |-- Call run_with_cancellation() -> full container lifecycle
  |     |-- complete() with result
  |
  +-- SSH path (workers configured): select_worker() round-robin with probe
        |-- deliver_manifest() via SCP
        |-- run_remote_job() via SSH
        |-- sync_state_back() via SCP
        |-- complete() with result
```

### Container Lifecycle (run_with_cancellation)

```
Phase 1: Load manifest from disk
Phase 2: Validate manifest
Phase 3: Ensure .assay/ in .gitignore
Phase 3+4: Connect to runtime provider (Docker/Compose/K8s)
Phase 5: provider.provision(manifest) -> ContainerId + optional container IP
Phase 5.5: Write Assay config + specs dir + per-session spec files into container
Phase 6: Write Assay run manifest into container (base64 exec)
Phase 7: Execute `assay run /tmp/smelt-manifest.toml --timeout N --base-branch REF`
          (streaming output via exec_streaming)
Phase 8: Collect results (git rev-parse, diff, branch create)
          K8s: fetch result branch from remote first
Phase 9: Create GitHub PR (if forge configured, changes exist, --no-pr not set)
Teardown: provider.teardown(container) -- always runs
```

### Event Flow (D177/D179/D186)

```
Assay (inside container) --POST--> /api/v1/events { job_id, payload }
  |
  v
post_event handler
  |-- Validate job exists
  |-- Strip control fields, build AssayEvent
  |-- ingest_event() -> EventStore ring buffer + EventBus broadcast + run_id cache
  |-- Extract manifest routing info
  |-- evaluate_notify_rules() (if phase == "complete")
  |     |-- Read source manifest [[notify]] rules
  |     |-- Match target_job against queued jobs
  |     |-- Build PeerUpdate with gate_summary, branch, source info
  |     |-- Skip terminal targets silently (D179)
  |
  v
Signal Delivery (per target session)
  |
  +-- HTTP-first (D186): POST to cached signal_url (container IP:7432)
  |     |-- 202 Accepted -> success, skip filesystem
  |     |-- Non-202 or error -> fall through to filesystem
  |
  +-- Filesystem fallback: write JSON to inbox directory
        <repo>/.assay/orchestrator/<run_id>/mesh/<session>/inbox/peer_update_<nanos>_<uuid>.json
        (atomic write via NamedTempFile + persist)
```

---

## Cross-Project Dependencies

### smelt-core -> assay-types

`smelt-core/Cargo.toml`:
```toml
assay-types = { path = "../../../crates/assay-types" }
```

Types imported from `assay-types`:
- `StateBackendConfig` -- Enum (`LocalFs`, `Linear { team_id, project_id }`) used for Assay state persistence configuration. Re-exported via `smelt_core::tracker::StateBackendConfig`. Serialized into the Assay RunManifest TOML as `[state_backend]` section.
- `signal::GateSummary` -- Struct `{ passed, failed, skipped }` for gate results in PeerUpdate signals.
- `signal::PeerUpdate` -- Struct `{ source_job, source_session, changed_files, gate_summary, branch }` for cross-job notifications.
- `signal::SignalRequest` -- Struct `{ target_session, update: PeerUpdate }` for signal delivery endpoints.

All three signal types are re-exported at `smelt_core` top level (`pub use assay_types::signal::*`). Design rule D012 mandates using canonical types from assay-types with no local mirrors.

### Impact of Changes

Changes to `assay-types` that affect `StateBackendConfig` or signal types require corresponding updates in:
1. `smelt-core/src/tracker.rs` (re-export)
2. `smelt-core/src/assay.rs` (SmeltRunManifest serialization)
3. `smelt-cli/src/serve/signals.rs` (signal delivery)
4. `smelt-cli/src/serve/notify.rs` (PeerUpdate construction)
5. `smelt-cli/src/serve/http_api.rs` (signal endpoint handler)

### Runtime Dependency: Assay Binary

`smelt run` provisions a container and invokes `assay run` inside it. The Assay binary must be available in the container image. The command is constructed by `AssayInvoker::build_run_command()`:
```
assay run /tmp/smelt-manifest.toml --timeout <max> --base-branch <ref>
```

Assay's signal server listens on port 7432 inside the container (constant `ASSAY_SIGNAL_PORT` in dispatch.rs). The Smelt server caches signal URLs as `http://<container_ip>:7432/api/v1/signal` for HTTP-first delivery.

---

## File Inventory

### smelt-core (`smelt/crates/smelt-core/src/`)

| File | Lines | Purpose |
|------|-------|---------|
| `lib.rs` | 67 | Module declarations, public re-exports |
| `error.rs` | 207 | `SmeltError` enum, `Result<T>` alias |
| `config.rs` | 197 | `SmeltConfig` from `.smelt/config.toml` |
| `manifest/mod.rs` | 365 | `JobManifest` and all sub-types, `resolve_repo_path()` |
| `manifest/validation.rs` | 245 | Semantic validation, cycle detection |
| `manifest/tests/` | 5 files | Tests organized by domain (core, compose, forge, kubernetes) |
| `provider.rs` | 145 | `RuntimeProvider` trait, `ContainerId`, `ExecHandle`, etc. |
| `docker.rs` | 754 | `DockerProvider`, resource parsing, host address detection |
| `compose.rs` | 904 | `ComposeProvider`, YAML generation, healthcheck polling |
| `k8s.rs` | 824 | `KubernetesProvider`, Pod/Secret spec generation |
| `forge.rs` | 463 | `ForgeClient` trait, `GitHubForge`, types |
| `collector.rs` | 366 | `ResultCollector<G: GitOps>`, `BranchCollectResult` |
| `monitor.rs` | 543 | `JobMonitor`, `JobPhase`, `RunState`, disk persistence |
| `assay.rs` | 932 | `AssayInvoker`, Assay serde types, event env computation |
| `tracker.rs` | 153 | `TrackerIssue`, `TrackerState`, `StateBackendConfig` re-export |
| `git/mod.rs` | 313 | `GitOps` trait (25+ methods), `GitWorktreeEntry`, `preflight()` |
| `git/cli/mod.rs` | 339 | `GitCli` implementation of `GitOps` |
| `git/cli/tests/` | 5 files | Git operation tests (basic, branch, commit, merge, worktree) |

### smelt-cli (`smelt/crates/smelt-cli/src/`)

| File | Lines | Purpose |
|------|-------|---------|
| `main.rs` | 100 | Binary entry point, tracing setup, command dispatch |
| `lib.rs` | 9 | Public module declarations |
| `commands/mod.rs` | 13 | Subcommand module declarations |
| `commands/init.rs` | 162 | Skeleton manifest generation |
| `commands/list.rs` | 152 | Past run enumeration |
| `commands/run/mod.rs` | 135 | `RunArgs`, `AnyProvider`, `execute()` |
| `commands/run/phases.rs` | 392 | Full container lifecycle with timeout/cancellation |
| `commands/run/dry_run.rs` | 186 | Validation + execution plan display |
| `commands/run/helpers.rs` | 156 | `should_create_pr()`, `ensure_gitignore_assay()` |
| `commands/serve.rs` | 267 | Daemon startup, component wiring |
| `commands/status.rs` | 313 | Job status display with stale PID detection |
| `commands/watch.rs` | 382 | PR polling loop with mock-friendly architecture |
| `serve/mod.rs` | 35 | Serve module declarations |
| `serve/config.rs` | 978 | `ServerConfig`, `WorkerConfig`, `AuthConfig`, `TrackerConfig` |
| `serve/types.rs` | 98 | `JobId`, `JobSource`, `JobStatus`, `QueuedJob` |
| `serve/queue.rs` | 475 | `ServerState`, persistence, concurrency control |
| `serve/dispatch.rs` | 664 | `dispatch_loop`, `run_job_task`, `run_ssh_job_task`, `select_worker` |
| `serve/http_api.rs` | 859 | Axum routes, auth middleware, SSE, signal delivery |
| `serve/events.rs` | 100 | `AssayEvent`, `EventStore` ring buffer, `EventBus` type |
| `serve/signals.rs` | 167 | `deliver_peer_update()`, `deliver_signal_http()`, path validation |
| `serve/notify.rs` | 310 | Cross-job PeerUpdate routing via `[[notify]]` rules |
| `serve/tui.rs` | 374 | Ratatui dashboard rendering, TUI thread management |
| `serve/tracker.rs` | 481 | `TrackerSource` trait, template loading, issue-to-manifest injection |
| `serve/tracker_poller.rs` | 540 | `AnyTrackerSource` enum, `TrackerPoller` background task |
| `serve/queue_watcher.rs` | 124 | `DirectoryWatcher` filesystem polling |
| `serve/ssh/mod.rs` | 114 | `SshClient` trait, `SshOutput`, re-exports |
| `serve/ssh/client.rs` | ~ | `SubprocessSshClient` implementation |
| `serve/ssh/operations.rs` | ~ | `deliver_manifest`, `run_remote_job`, `sync_state_back` |
| `serve/ssh/mock.rs` | ~ | `MockSshClient` test double |
| `serve/github/` | 4 files | GitHub Issues integration (`GhClient` trait, subprocess `gh` CLI) |
| `serve/linear/` | 4 files | Linear Issues integration (GraphQL API via reqwest) |
| `serve/tests/` | 8 files | Serve module integration tests |
