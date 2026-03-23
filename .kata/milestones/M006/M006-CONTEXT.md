# M006: Parallel Dispatch Daemon — Context

**Gathered:** 2026-03-23
**Status:** Ready for planning

## Project Description

Smelt is the infrastructure layer in the smelt/assay/cupel agentic development toolkit. M001–M005 delivered single-job execution across three runtimes (Docker, Compose, Kubernetes), GitHub PR lifecycle, per-job state tracking, and a stable `smelt-core` library API. M006 adds `smelt serve` — a long-running daemon that accepts a queue of manifests and dispatches up to N concurrent `smelt run` sessions, each isolated in its own container/Pod, with a Ratatui TUI for live observability.

## Why This Milestone

Every smelt run today is a one-shot CLI invocation. Dispatching work in parallel currently means running multiple terminal processes manually and checking `smelt status` per job. `smelt serve` closes this gap: it becomes the long-running infrastructure host, and manifests are submitted to it (via file drop or HTTP API) rather than being invoked directly. This is the natural evolution of R023 (parallel K8s session orchestration) into a runtime-agnostic dispatch layer, and makes Smelt usable as a headless server-side infrastructure component.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Run `smelt serve --config server.toml` and leave it running as a long-running daemon
- Drop a manifest TOML into the configured `queue_dir` and watch it get picked up, dispatched, and completed — without touching the CLI
- POST a manifest to `POST /api/v1/jobs` and receive a job ID back; poll `GET /api/v1/jobs/:id` for status
- See all running and queued jobs, their current phase, elapsed time, runtime, and exit status in a live Ratatui TUI in the terminal where `smelt serve` is running
- Configure `max_concurrent: N` and watch smelt serve respect the cap — jobs wait in queue when N are already running
- Configure `retry_attempts: N` and see failed jobs automatically retried (up to N times) before moving to permanent failure
- Run `smelt run manifest.toml` directly as before — zero behavior change on the single-job path

### Entry point / environment

- Entry point: `smelt serve --config server.toml` (new subcommand)
- Environment: local dev or headless server; Docker/Compose/K8s available per manifest
- Live dependencies: Docker daemon (for docker/compose runtimes), kubeconfig (for k8s runtime)

## Completion Class

- Contract complete means: `ServerConfig` parses and validates; `JobQueue` unit tests prove FIFO + concurrency cap; HTTP API request/response types serialize correctly; TUI renders with mock data
- Integration complete means: `smelt serve` dispatches real `smelt run` sessions concurrently; directory watch picks up dropped manifests; HTTP API accepts and responds; TUI reflects live state transitions
- Operational complete means: `smelt serve` runs for the full lifecycle of multiple overlapping jobs without leaked processes or orphaned containers; Ctrl+C triggers graceful shutdown (running jobs torn down); retry fires on failure with backoff

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- Drop 3 manifests into `queue_dir` with `max_concurrent: 2` — two dispatch immediately, one queues; second batch starts after first completes
- POST a manifest to `/api/v1/jobs`, poll `/api/v1/jobs/:id`, see it move through phases to completion
- TUI renders all job states in real time, updating phase and elapsed time without corruption
- Ctrl+C during active job dispatch cleanly tears down all running containers/Pods (no `docker ps` orphans after exit)
- Auto-retry fires for a deliberately-failing manifest, increments attempt count, eventually moves to `failed`
- `smelt run manifest.toml` still works exactly as before — no regressions in the full test suite

## Risks and Unknowns

- **Process-per-job architecture vs in-process execution** — `smelt run` today is a single async `run_with_cancellation()` call. Running multiple jobs concurrently could mean: (a) spawn a subprocess per job, or (b) run multiple `run_with_cancellation()` futures concurrently in one tokio runtime. Option (a) is simpler for isolation and cleanup but loses the library benefit; option (b) is cleaner but requires `run.rs` execution to be truly Send + isolatable. Needs decision before S01.
- **Ratatui TUI in a tokio async context** — Ratatui draws synchronously to stdout; `smelt serve` will be driving jobs on tokio tasks concurrently. The standard pattern is a background thread for the TUI event loop with an `Arc<Mutex<AppState>>` shared with the tokio tasks. Need to verify this pattern works correctly with the existing `tracing` subscriber (which also writes to stderr).
- **Graceful shutdown across multiple concurrent jobs** — Ctrl+C must trigger teardown for all running containers/Pods. With the existing single-job path, cancellation is a tokio oneshot. With N concurrent jobs, this becomes a broadcast. Need to choose between `tokio::sync::broadcast`, `CancellationToken` (tokio-util), or a shared `AtomicBool`. The existing D037 (generic future for cancellation) needs revisiting.
- **HTTP API + Ratatui TUI co-existing on the same process** — Both need tokio tasks; the TUI event loop is typically a blocking thread. The interaction between `crossterm`'s terminal raw mode and `axum`/`hyper` on the same process needs validation.

## Existing Codebase / Prior Art

- `crates/smelt-cli/src/commands/run.rs` — `run_with_cancellation()` is the core execution function; `AnyProvider` enum covers all 3 runtimes; this is the unit that `smelt serve` will dispatch per-job
- `crates/smelt-core/src/monitor.rs` — `JobMonitor` + `RunState` + `JobPhase` are the state persistence layer; `smelt serve` will use the same state files so `smelt status <job>` still works
- `crates/smelt-core/src/manifest.rs` — `JobManifest` is the input contract; `smelt serve` reads the same TOML format
- `crates/smelt-cli/src/commands/status.rs` — `smelt status` reads per-job state; will remain unchanged
- `crates/smelt-cli/src/commands/watch.rs` — `smelt watch` is the single-job polling loop; `smelt serve` does not replace this
- `crates/smelt-core/src/error.rs` — `SmeltError` error types; `smelt serve` will add `ServerError` variants or reuse existing patterns

> See `.kata/DECISIONS.md` for all architectural and pattern decisions. D004 (RuntimeProvider trait), D023 (teardown guarantee), D036/D037 (cancellation abstraction), D084 (AnyProvider dispatch) are the most relevant.

## Relevant Requirements

- R023 — Parallel K8s session orchestration (now generalized to all runtimes): M006 delivers this at the infrastructure level
- R024 — `smelt serve` HTTP API: new active requirement created for M006

## Scope

### In Scope

- `smelt serve` subcommand with `--config server.toml` and CLI flag overrides
- `ServerConfig` TOML: `queue_dir`, `max_concurrent`, `retry_attempts`, `retry_backoff_secs`, `server.port`, `server.host`
- Directory watcher: polls `queue_dir` for new `.toml` files, enqueues them, moves to `queue_dir/dispatched/` after pickup
- HTTP API: `POST /api/v1/jobs` (accept manifest TOML body, return `{ job_id }`), `GET /api/v1/jobs` (list all jobs), `GET /api/v1/jobs/:id` (single job state), `DELETE /api/v1/jobs/:id` (cancel queued job)
- `JobQueue`: in-memory FIFO with concurrency cap; jobs move through `Queued → Running → Complete/Failed/Retrying`
- Auto-retry with exponential backoff up to `retry_attempts`; retry count tracked in job state
- Ratatui TUI: live table of all jobs (id, manifest name, runtime, phase, attempt, elapsed, exit status); updates from a shared `Arc<Mutex<ServerState>>`
- Graceful shutdown: Ctrl+C broadcasts cancellation to all running jobs, waits for teardown, exits
- `smelt serve` writes per-job state files to `.smelt/runs/` so `smelt status <job>` still works
- `examples/server.toml` — documented server config example

### Out of Scope / Non-Goals

- Linear/GitHub Issues integration for job sourcing (deferred to a future milestone)
- SSH worker pools / remote dispatch (deferred)
- `workspace.isolation: docker` for smelt serve itself — jobs use whatever runtime their manifest specifies
- Job priorities or dependency graphs between jobs
- Persistent queue across `smelt serve` restarts (queue is in-memory only; manifests already on disk in queue_dir are re-picked up on restart)
- Web UI beyond the JSON API
- Windows host support
- crates.io publish

## Technical Constraints

- D036/D037: cancellation must be a broadcast across N jobs, not a single oneshot — revisit D037's "generic future" pattern; `tokio_util::CancellationToken` is the likely answer (D037 was explicitly marked "revisable if CancellationToken needed across task trees")
- D004: all job execution goes through `RuntimeProvider` trait — no special-casing by runtime in the daemon
- D023: teardown guarantee is non-negotiable — every job that starts a container must tear it down, even on panic or Ctrl+C
- Ratatui TUI + tokio: TUI event loop must be a `std::thread::spawn` background thread; state shared via `Arc<Mutex<ServerState>>`
- HTTP API: use `axum` (already present as a possible dep via hyper in tokio ecosystem) or `warp`; prefer `axum` for ergonomics
- `smelt run manifest.toml` single-job path: must remain 100% unchanged — smelt serve is additive

## Integration Points

- **`run_with_cancellation()`** — per-job execution function; smelt serve calls this (or a wrapper) per dispatched job on a tokio task
- **`JobMonitor` / `RunState`** — smelt serve writes job state so `smelt status` and `smelt watch` still work per-job
- **Directory watcher** — `notify` crate (cross-platform filesystem events) or polling; pick based on platform support needs
- **HTTP server** — `axum` crate for the REST API; runs as a tokio task alongside the job dispatch loop
- **Ratatui** — TUI library for the terminal dashboard; background thread, `Arc<Mutex<ServerState>>` for state sharing
- **`tokio_util::CancellationToken`** — broadcast cancellation to all running job tasks on Ctrl+C

## Open Questions

- **Subprocess vs in-process execution** — should each job be a `tokio::spawn` task calling `run_with_cancellation()`, or a `std::process::Command` spawning `smelt run`? In-process is more efficient and allows direct state sharing; subprocess gives stronger isolation and simpler cleanup on crash. Current leaning: in-process (tokio tasks) with `CancellationToken` for teardown, since `run_with_cancellation()` is already designed for this. Subprocess is a fallback if in-process proves too complex for multi-job teardown.
- **Queue state on restart** — manifests in `queue_dir` (not yet moved to `dispatched/`) will be re-picked up on restart. Is this the right behavior? Alternative: track a `queue_dir/.dispatched-log` to avoid re-dispatching already-started jobs. Current leaning: simple file-move semantics (pickup = move), no restart recovery of in-flight jobs.
- **axum version** — `axum` is not currently in smelt's dependency tree. Need to check compatible version with existing tokio workspace dep before S01.
