# Architecture: Smelt

Smelt is a containerized job execution engine for agentic development workflows. It provisions containers, injects Assay run manifests, executes `assay run` inside the container, collects results, and optionally creates pull requests. Two crates split the library from the daemon.

---

## Overview

Smelt solves the problem of running Assay specs in isolated, reproducible environments. A job manifest declares which specs to run, which container image to use, resource limits, credentials, and delivery targets. Smelt provisions the container, writes the manifest and spec files into it, executes the Assay pipeline, collects git results, and optionally creates a GitHub PR.

In daemon mode (`smelt serve`), Smelt becomes a job dispatch server that accepts work from three sources (HTTP API, filesystem directory watch, issue tracker polling), queues jobs, dispatches them to local Docker or remote SSH workers, and provides a live TUI dashboard.

---

## Two-Crate Split

```
smelt-core (library)              smelt-cli (binary)
  Domain types                      CLI entry point (clap)
  Manifest parsing + validation     Subcommand dispatch
  RuntimeProvider trait              AnyProvider enum dispatch
  DockerProvider                     serve module (daemon)
  ComposeProvider                      HTTP API (axum)
  KubernetesProvider                   Queue + persistence
  ForgeClient trait                    Dispatch loop
  GitHubForge                          SSH worker pools
  GitOps trait + GitCli                Directory watcher
  ResultCollector                      Tracker poller
  JobMonitor                           TUI (ratatui)
  AssayInvoker                         Event bus + SSE
  Tracker types                        Signal delivery
```

**smelt-core** is a standalone library with no CLI dependencies. It exposes the `RuntimeProvider` trait, concrete providers (Docker, Compose, Kubernetes), forge integration, git operations, manifest parsing, and job monitoring. It depends on `assay-types` for `StateBackendConfig` and signal types.

**smelt-cli** is the binary crate. It depends on `smelt-core` with the `forge` feature enabled. It adds daemon mode with HTTP API, queue persistence, SSH worker dispatch, TUI dashboard, tracker polling, and event ingestion/signal delivery infrastructure.

The split ensures smelt-core can be used as a library (e.g., embedded in other tools) without pulling in the daemon infrastructure.

---

## Key Abstractions

### RuntimeProvider

The core abstraction for container execution backends. Defines the full lifecycle of a containerized job.

```
trait RuntimeProvider: Send + Sync {
    async fn provision(manifest) -> Result<ProvisionResult>
    async fn exec(container_id, command) -> Result<ExecHandle>
    async fn exec_streaming(container_id, command) -> Result<ExecHandle>
    async fn collect(container_id) -> Result<CollectResult>
    async fn teardown(container_id) -> Result<()>
}
```

| Implementation | Backing Technology | Characteristics |
|----------------|-------------------|-----------------|
| `DockerProvider` | bollard (Docker API) | Single container, resource limits, host address detection |
| `ComposeProvider` | Docker Compose subprocess + bollard exec | Multi-service stacks, YAML generation, healthcheck polling |
| `KubernetesProvider` | kube client (Pod + Secret) | Pod/Secret lifecycle, WebSocket exec, namespace isolation |

`AnyProvider` in smelt-cli is an enum dispatcher that avoids `Box<dyn RuntimeProvider>` (not object-safe due to RPITIT async methods). It dispatches based on `manifest.environment.runtime`.

### GitOps

Trait with 25+ async methods for git operations. Production implementation `GitCli` shells out to the `git` binary via `tokio::process::Command`. Exists as a test seam.

Key operations: worktree management, branch create/delete, merge (including squash), rev-parse, diff, fetch, status (porcelain parsing).

### ForgeClient

PR lifecycle operations. Feature-gated behind the `forge` Cargo feature.

```
trait ForgeClient {
    async fn create_pr(config, title, body, head, base) -> Result<PrHandle>
    async fn poll_pr_status(config, pr_number) -> Result<PrStatus>
}
```

Single implementation: `GitHubForge` using octocrab.

### TrackerSource

RPITIT-based async trait for external issue trackers. Two methods: `poll_ready_issues()` and `transition_state()`. Not object-safe due to RPITIT, so solved via `AnyTrackerSource` enum dispatch.

| Implementation | Backing Technology |
|----------------|-------------------|
| `GithubTrackerSource` | GitHub Issues API (via `gh` CLI subprocess) |
| `LinearTrackerSource` | Linear GraphQL API (via reqwest) |
| `MockTrackerSource` | VecDeque-based test double |

### SshClient

Async SSH operations for remote worker dispatch.

```
trait SshClient: Send + Sync {
    async fn exec(host, command) -> Result<SshOutput>
    async fn probe(host) -> Result<()>
    async fn scp_to(host, local, remote) -> Result<()>
    async fn scp_from(host, remote, local) -> Result<()>
}
```

Production implementation: `SubprocessSshClient` (shells out to system `ssh`/`scp`). Uses `-o ConnectTimeout` for fast-fail probing.

---

## Domain Model

### JobManifest

The central configuration type. Loaded from TOML with strict `deny_unknown_fields`.

```
JobManifest
  ├── JobMeta              — name, repo, base_ref
  ├── Environment          — runtime (docker/compose/kubernetes), image, resources
  ├── CredentialConfig     — provider, model, env var mappings
  ├── SessionDef[]         — name, spec, harness, timeout, depends_on
  ├── MergeConfig          — strategy, order, ai_resolution, target
  ├── ForgeConfig?         — provider, repo, token_env
  ├── KubernetesConfig?    — namespace, context, ssh_key_env, cpu/mem limits
  ├── ComposeService[]     — name, image, extra (passthrough fields)
  ├── StateBackendConfig?  — from assay-types (LocalFs, Linear, GitHub, etc.)
  ├── NotifyRule[]         — target_job, on_session_complete
  └── runtime_env          — computed at dispatch time, not serialized
```

### ServerState

Daemon-side state managing the job queue, event bus, and signal routing.

```
ServerState
  ├── jobs: VecDeque<QueuedJob>     — ordered job queue
  ├── running_count, max_concurrent — concurrency control
  ├── queue_dir                     — persistence path
  ├── round_robin_idx               — SSH worker selection
  ├── event_bus                     — broadcast::Sender<AssayEvent>
  ├── events: HashMap<String, EventStore>  — per-job event ring buffers
  ├── run_ids: HashMap<String, String>     — job_id → run_id mapping
  └── signal_urls: HashMap<String, String> — session → signal URL cache
```

### QueuedJob

```
QueuedJob
  ├── id: JobId
  ├── manifest_path: PathBuf
  ├── source: JobSource { DirectoryWatch, HttpApi, Tracker }
  ├── status: JobStatus { Queued, Dispatching, Running, Retrying, Complete, Failed }
  ├── attempt: u32
  ├── queued_at, started_at: u64
  └── worker_host: Option<String>
```

### Type Hierarchies

| Type | Variants | Purpose |
|------|----------|---------|
| `JobPhase` | 10 variants (LoadManifest through Teardown) | Container lifecycle progress |
| `RunState` | Running, Complete, Failed | Job terminal state |
| `JobSource` | DirectoryWatch, HttpApi, Tracker | How the job was submitted |
| `JobStatus` | Queued, Dispatching, Running, Retrying, Complete, Failed | Queue lifecycle |
| `TrackerState` | Ready, Queued, Running, PrCreated, Done, Failed | Issue tracker lifecycle |
| `PrState` | Open, Merged, Closed | PR lifecycle |
| `CiStatus` | Pending, Success, Failure | PR CI check status |

---

## Container Lifecycle

The full container lifecycle (`run_with_cancellation`) proceeds through these phases:

| Phase | Action | Provider Method |
|-------|--------|----------------|
| 1 | Load manifest from disk | -- |
| 2 | Validate manifest | -- |
| 3 | Ensure `.assay/` in `.gitignore` | -- |
| 4 | Connect to runtime provider | -- |
| 5 | Provision container | `provider.provision()` |
| 5.5 | Write Assay config + specs into container | base64-encoded exec |
| 6 | Write run manifest into container | base64-encoded exec |
| 7 | Execute `assay run` with streaming output | `provider.exec_streaming()` |
| 8 | Collect results (git state, branch creation) | `ResultCollector` |
| 9 | Create GitHub PR (if configured, changes exist) | `ForgeClient.create_pr()` |
| Teardown | Stop and remove container (always runs) | `provider.teardown()` |

Cancellation is propagated via `tokio_util::CancellationToken`. Teardown runs regardless of success or cancellation.

---

## Daemon Mode

### Five Concurrent Components

`smelt serve` runs five components under `tokio::select!`:

| Component | Responsibility | Mechanism |
|-----------|---------------|-----------|
| **dispatch_loop** | Poll queue every 2s, spawn local or SSH job tasks | `ServerState::try_dispatch()` |
| **DirectoryWatcher** | Watch `queue_dir/` for `.toml` manifest files | Filesystem polling, rename to `dispatched/` |
| **axum::serve** | HTTP API on configurable host:port | TCP listener, auth middleware |
| **TrackerPoller** | Poll GitHub Issues or Linear for `smelt:ready` labels | Configurable interval, template injection |
| **Ctrl+C handler** | Broadcast cancellation to all in-flight jobs | `CancellationToken` |

Optional TUI runs on a dedicated `std::thread` (crossterm requires blocking I/O), coordinated via `Arc<AtomicBool>`.

### Job Dispatch

Three submission paths converge on `ServerState.enqueue()`:

1. **HTTP API**: `POST /api/v1/jobs` with TOML body, parsed and validated
2. **Directory Watch**: `.toml` file appears in `queue_dir/`, renamed to `dispatched/`
3. **Tracker Poller**: Issues with `smelt:ready` label transitioned to Queued, manifest injected from template

The dispatch loop promotes `Queued` jobs to `Dispatching`, then:

- **Local path** (no SSH workers): spawns `run_job_task()` which runs the full container lifecycle
- **SSH path** (workers configured): round-robin worker selection with probe, SCP manifest delivery, SSH remote execution, SCP state sync back

Queue state is persisted to `.smelt-queue-state.toml` for crash recovery. On restart, `Running` jobs are remapped to `Queued`.

---

## HTTP API

Seven endpoints served by axum with optional bearer token authentication.

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| `GET` | `/health` | None | Health check (`{"status":"ok"}`) |
| `POST` | `/api/v1/jobs` | Write | Submit job (TOML body), returns `{"job_id": "..."}` |
| `GET` | `/api/v1/jobs` | Read | List all jobs as JSON array |
| `GET` | `/api/v1/jobs/{id}` | Read | Get single job status |
| `DELETE` | `/api/v1/jobs/{id}` | Write | Cancel queued job (409 if running) |
| `POST` | `/api/v1/events` | Write | Ingest Assay event (64KB limit) |
| `GET` | `/api/v1/events` | Read | SSE stream of events (`?job=<id>` filter) |
| `POST` | `/api/v1/jobs/{id}/signals` | Write | Deliver PeerUpdate signal to session (64KB limit) |

### Event Flow

```
Assay (inside container) --POST--> /api/v1/events { job_id, payload }
  → Validate job exists
  → Build AssayEvent, ingest to EventStore ring buffer + broadcast
  → Cache run_id and signal_url
  → On phase "complete": evaluate [[notify]] rules
    → Match target_job, build PeerUpdate, deliver signal

Signal Delivery (per target session):
  → HTTP-first: POST to cached signal_url (container_ip:7432)
    → 202 Accepted → success
    → Non-202 or error → filesystem fallback
  → Filesystem fallback: atomic write to inbox directory
```

### Authentication

Two-tier token auth via `[auth]` config section:
- `write_token_env` -- Required for POST/DELETE endpoints
- `read_token_env` -- Required for GET endpoints (except `/health`)

Tokens reference environment variable names, not literal values.

---

## TUI Dashboard

Split layout rendered by ratatui:
- **Top pane**: Job table with columns for Job ID, Manifest, Source, Status, Attempt, Elapsed, Worker
- **Bottom pane**: Event pane showing the most recent 20 events across all jobs
- **Source column**: Shows submission origin (`HTTP`, `DirWatch`, `Tracker`)
- **Keyboard**: `q` or `Ctrl+C` to quit

Runs on a dedicated `std::thread` because crossterm requires blocking terminal I/O. Coordinated with the async runtime via `Arc<AtomicBool>` for shutdown signaling. TUI mode redirects tracing logs to `.smelt/serve.log` via `tracing-appender`.

---

## Configuration

### SmeltConfig (`.smelt/config.toml`)

Project-level defaults. Returns sensible defaults if the file is missing.

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `default_image` | String | `"ubuntu:22.04"` | Default container image |
| `credential_sources` | HashMap | `{}` | Logical name to env var mapping |
| `default_resources` | HashMap | `{}` | CPU/memory defaults |
| `default_timeout` | u64 | `600` | Session timeout in seconds |

### ServerConfig (`server.toml`)

Daemon configuration with comprehensive validation.

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `queue_dir` | PathBuf | (required) | Queue persistence directory |
| `max_concurrent` | usize | (required, >= 1) | Max parallel jobs |
| `retry_attempts` | u32 | `3` | Max retry count |
| `retry_backoff_secs` | u64 | `5` | Backoff base (reserved) |
| `ssh_timeout_secs` | u64 | `3` | SSH connection timeout |
| `server.host` | String | `"127.0.0.1"` | HTTP bind address |
| `server.port` | u16 | `8765` | HTTP port |
| `[[workers]]` | Vec | `[]` | SSH worker pool definitions |
| `[auth]` | Optional | None | Bearer token auth config |
| `[tracker]` | Optional | None | Issue tracker polling config |

### Job Manifests (`job-manifest.toml`)

Per-job configuration with strict `deny_unknown_fields` on all sections.

Key sections: `[job]`, `[environment]`, `[credentials]`, `[[session]]`, `[merge]`, `[forge]` (optional), `[kubernetes]` (optional), `[[services]]` (optional), `[state_backend]` (optional), `[[notify]]` (optional).

### Environment Variables

| Variable | Purpose |
|----------|---------|
| `SMELT_LOG` / `RUST_LOG` | Tracing filter |
| `SMELT_EVENT_HOST` | Override host address for container event injection |
| `SMELT_EVENT_URL` | Computed: injected into container environment |
| `SMELT_JOB_ID` | Job identifier injected into container |
| `SMELT_WRITE_TOKEN` | Auth token for event POST (when auth configured) |
| `ANTHROPIC_API_KEY` etc. | Credential env vars mapped via `credentials.env` |
| `LINEAR_API_KEY` | Linear API key (tracker integration) |

---

## Error Handling

### SmeltError

`#[non_exhaustive]` enum with 10 variants:

| Variant | Fields | Description |
|---------|--------|-------------|
| `GitNotFound` | -- | `git` not on PATH |
| `NotAGitRepo` | -- | Working directory is not a git repository |
| `GitExecution` | `operation`, `message` | Git command failed |
| `MergeConflict` | `session`, `files` | Merge conflict detected during result collection |
| `Manifest` | `field`, `message` | Parse or validation error |
| `Provider` | `operation`, `message`, `source?` | Runtime provider failure (Docker/Compose/K8s) |
| `Forge` | `operation`, `message` | GitHub API failure |
| `Tracker` | `operation`, `message` | Issue tracker API failure |
| `Credential` | `provider`, `message` | Credential resolution failure |
| `Config` | `path`, `message` | Config load/parse failure |
| `Io` | `operation`, `path`, `source` | I/O with context |

Convenience constructors (`SmeltError::io()`, `::provider()`, `::forge()`, etc.) ensure consistent context attachment.

### CLI Error Handling

smelt-cli uses `anyhow::Result` for command-level errors, wrapping `SmeltError` via `.with_context()`. Commands return `Result<i32>` where the integer is the process exit code.

---

## Cross-Project Dependency on assay-types

smelt-core depends on `assay-types` via path dependency:

```toml
assay-types = { path = "../../../crates/assay-types" }
```

### Types Imported

| Type | Module | Purpose |
|------|--------|---------|
| `StateBackendConfig` | `assay_types` | Enum for Assay state persistence configuration |
| `GateSummary` | `assay_types::signal` | Gate results in PeerUpdate signals |
| `PeerUpdate` | `assay_types::signal` | Cross-job notification payload |
| `SignalRequest` | `assay_types::signal` | Signal delivery envelope |

All signal types are re-exported at `smelt_core` top level (`pub use assay_types::signal::*`). Design rule D012 mandates using canonical types from assay-types with no local mirrors.

### Impact of Changes

Changes to `StateBackendConfig` or signal types in assay-types require corresponding updates in:

1. `smelt-core/src/tracker.rs` (re-export)
2. `smelt-core/src/assay.rs` (SmeltRunManifest serialization)
3. `smelt-cli/src/serve/signals.rs` (signal delivery)
4. `smelt-cli/src/serve/notify.rs` (PeerUpdate construction)
5. `smelt-cli/src/serve/http_api.rs` (signal endpoint handler)

### Runtime Dependency

`smelt run` provisions a container and invokes `assay run` inside it. The Assay binary must be available in the container image. The command is constructed by `AssayInvoker::build_run_command()`:

```
assay run /tmp/smelt-manifest.toml --timeout <max> --base-branch <ref>
```

Assay's signal server listens on port 7432 inside the container (constant `ASSAY_SIGNAL_PORT`). Smelt caches signal URLs as `http://<container_ip>:7432/api/v1/signal` for HTTP-first delivery.
