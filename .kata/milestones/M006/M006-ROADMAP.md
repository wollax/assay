# M006: Parallel Dispatch Daemon

**Vision:** `smelt serve` is a long-running daemon that accepts a queue of job manifests (via directory watch or HTTP POST), dispatches up to N concurrent `smelt run` sessions each isolated in their own container/Pod, auto-retries failures, and surfaces live state in a Ratatui TUI — all while leaving the existing `smelt run` single-job path completely unchanged.

## Success Criteria

- `smelt serve --config server.toml` starts, loads config, and begins watching `queue_dir` for manifest files
- Dropping 3 manifests into `queue_dir` with `max_concurrent: 2` dispatches two immediately and queues the third; the third dispatches as soon as one completes
- `POST /api/v1/jobs` with a manifest TOML body enqueues a job and returns `{ "job_id": "..." }`; `GET /api/v1/jobs/:id` returns the current job state as JSON
- The Ratatui TUI shows a live table of all queued, running, and completed jobs, updating phase and elapsed time in real time
- A job that fails auto-retries up to `retry_attempts` with backoff before moving to permanent `failed`
- Ctrl+C during active dispatch cleanly tears down all running containers/Pods — `docker ps` shows no orphans after exit
- `smelt run manifest.toml` direct invocation is completely unchanged — zero regressions in `cargo test --workspace`
- `smelt status <job>` still reads per-job state written by `smelt serve`

## Key Risks / Unknowns

- **In-process vs subprocess per-job architecture** — running N concurrent `run_with_cancellation()` futures on one tokio runtime requires all job state to be truly isolated and Send. If any shared state leaks across jobs (e.g. tracing subscriber, global Docker client), correctness breaks. Must be proven in S01 before building the full queue.
- **Ratatui TUI + tokio + axum on the same process** — crossterm puts the terminal into raw mode; axum + tokio drive async I/O; tracing writes to stderr. All three must coexist without corrupting terminal output or blocking. Needs integration proof before S03.
- **Broadcast cancellation across N tokio tasks** — D037's generic-future cancellation was designed for one job. Broadcasting to N concurrent jobs requires `tokio_util::CancellationToken` (D037 was marked "revisable if CancellationToken needed across task trees"). Must prove teardown fires for all jobs before claiming Ctrl+C works.

## Proof Strategy

- **In-process multi-job correctness** → retire in S01 by running two concurrent real `run_with_cancellation()` calls in a single tokio runtime in an integration test — both must complete independently with correct per-job state and no cross-contamination.
- **TUI + tokio + axum coexistence** → retire in S03 by running `smelt serve` with 2 concurrent jobs and verifying TUI renders clean output throughout, HTTP API responds, and tracing doesn't corrupt the display.
- **Broadcast cancellation** → retire in S02 by integration-testing Ctrl+C (simulate via CancellationToken::cancel()) while 2 jobs are running — both must call teardown and exit cleanly with no orphaned containers.

## Verification Classes

- Contract verification: `ServerConfig` roundtrip + validation tests; `JobQueue` unit tests (FIFO order, concurrency cap, retry state machine, cancel queued job); HTTP API request/response shape tests (JSON serialization); TUI `ServerState` rendering unit tests with mock data
- Integration verification: directory watcher picks up dropped manifests end-to-end; HTTP POST dispatches a real job; two concurrent jobs run and complete independently; retry fires and increments attempt count; `smelt status <job>` reads state written by `smelt serve`
- Operational verification: Ctrl+C with 2 active jobs → clean teardown + no orphans; restart with manifests in queue_dir → re-picked up; TUI + axum coexistence for the full lifecycle of a job
- UAT / human verification: `smelt serve` run manually with 3 real manifests, observing TUI live updates and HTTP API responses; Ctrl+C teardown confirmed visually

## Milestone Definition of Done

This milestone is complete only when all are true:

- `smelt serve --config server.toml` is a working subcommand that dispatches real jobs
- Directory watch and HTTP POST both enqueue jobs successfully
- `max_concurrent` cap is enforced — jobs queue and drain correctly
- Auto-retry with backoff fires for failed jobs up to `retry_attempts`
- Ratatui TUI renders live job state correctly throughout the full job lifecycle
- HTTP API (`/api/v1/jobs`) returns correct JSON state for queued, running, and completed jobs
- Ctrl+C triggers clean teardown of all running containers/Pods with no orphans
- `smelt run manifest.toml` is unchanged — `cargo test --workspace` is green
- `smelt status <job>` reads per-job state written by `smelt serve`
- `examples/server.toml` ships as the canonical documented server config

## Requirement Coverage

- Covers: R023 (parallel orchestration — generalized to all runtimes), R024 (smelt serve HTTP API), R025 (live TUI observability)
- Partially covers: none
- Leaves for later: R026 (Linear/GitHub Issues backlog integration for job sourcing), R027 (SSH worker pools / remote dispatch), R028 (persistent queue across restarts)
- Orphan risks: none

## Slices

- [x] **S01: JobQueue + In-Process Dispatch** `risk:high` `depends:[]`
  > After this: `JobQueue` unit-tested with real concurrency cap; two real `smelt run` jobs dispatch concurrently in a tokio runtime integration test; `CancellationToken` broadcast teardown confirmed; `smelt run` regressions: zero.

- [x] **S02: Directory Watch + HTTP API** `risk:medium` `depends:[S01]`
  > After this: `smelt serve` accepts jobs via directory watch (drop a `.toml` → job dispatches) and via `POST /api/v1/jobs`; `GET /api/v1/jobs` and `GET /api/v1/jobs/:id` return live JSON state; cancel queued job via `DELETE /api/v1/jobs/:id`; integration tests for both ingress paths pass.

- [x] **S03: Ratatui TUI + Server Config + Graceful Shutdown** `risk:medium` `depends:[S01,S02]`
  > After this: `smelt serve --config server.toml` runs with a live Ratatui TUI showing all jobs, phases, elapsed time, and attempt counts; Ctrl+C tears down all running containers cleanly; `ServerConfig` validates and parses; `examples/server.toml` ships; `cargo test --workspace` all green.

## Boundary Map

### S01 → S02, S03

Produces:
- `JobQueue` struct: `Arc<Mutex<VecDeque<QueuedJob>>>` with `enqueue(manifest: JobManifest, source: JobSource) -> JobId`, `try_dispatch() -> Option<RunningJob>`, `complete(id, outcome)`, `cancel(id) -> bool`, `retry_eligible(id) -> bool` — all unit-tested
- `QueuedJob { id: JobId, manifest: JobManifest, source: JobSource, attempt: u32, status: JobStatus, queued_at: Instant }` — the primary in-memory job record
- `JobStatus` enum: `Queued | Dispatching | Running | Retrying | Complete | Failed` with `#[derive(Serialize, Clone)]`
- `JobId` opaque String wrapper (format: `<job-name>-<uuid-v4>`)
- `ServerState: Arc<Mutex<ServerState>>` — shared mutable state for all jobs; updated by dispatch loop and read by TUI/HTTP handlers
- `dispatch_loop(state, config, cancel_token)` async function — main dispatch engine: polls queue, spawns tokio tasks, respects `max_concurrent`, handles retry backoff, writes per-job `RunState` to `.smelt/runs/`
- `run_job_task(job, state, cancel_token)` async function — wraps `run_with_cancellation()` per job; updates `ServerState` on phase transitions
- `CancellationToken` (tokio-util) added as workspace dep — broadcast cancellation from Ctrl+C to all running job tasks
- Integration test: two concurrent real docker jobs (or mock providers) dispatch, complete, state written, `smelt status <job>` reads correctly

Consumes:
- `run_with_cancellation()` from `crates/smelt-cli/src/commands/run.rs` (or extracted to `smelt-core`)
- `JobMonitor` / `RunState` from `crates/smelt-core/src/monitor.rs`
- `JobManifest` from `crates/smelt-core/src/manifest.rs`
- `AnyProvider` dispatch from `crates/smelt-cli/src/commands/run.rs`

### S02 → S03

Produces:
- `DirectoryWatcher`: polls `queue_dir` every 2s; moves discovered `.toml` files to `queue_dir/dispatched/<timestamp>-<name>.toml` after enqueue; calls `JobQueue::enqueue()`
- HTTP API via `axum`: `POST /api/v1/jobs` (accept `text/plain` TOML body, parse `JobManifest`, enqueue, return `{ "job_id": "..." }`), `GET /api/v1/jobs` (array of all job states), `GET /api/v1/jobs/:id` (single job state or 404), `DELETE /api/v1/jobs/:id` (cancel queued job or 409 if already running)
- `JobStateResponse` serde type: `{ id, manifest_name, runtime, status, phase, attempt, queued_at_secs, started_at_secs?, elapsed_secs?, exit_code? }`
- `axum` added to `smelt-cli/Cargo.toml`; `tower` + `tower-http` for middleware
- Integration tests: drop a real manifest TOML into a temp `queue_dir`, assert job enqueued and dispatched; POST manifest body, assert 200 + job_id; GET jobs, assert job appears; DELETE queued job, assert 200; DELETE running job, assert 409

Consumes from S01:
- `JobQueue`, `ServerState`, `JobId`, `JobStatus`, `QueuedJob`

### S03 (final wiring — no new public surfaces)

Produces:
- `ServerConfig` TOML struct: `queue_dir: PathBuf`, `max_concurrent: usize`, `retry_attempts: u32`, `retry_backoff_secs: u64`, `[server] host: String`, `[server] port: u16`; parsed from `server.toml` + CLI flag overrides
- `smelt serve` subcommand in `main.rs`: loads `ServerConfig`, starts `dispatch_loop`, `DirectoryWatcher`, `axum` HTTP server, and Ratatui TUI — all as concurrent tokio tasks + one background thread for TUI
- Ratatui TUI: `crossterm` backend, background `std::thread::spawn`; draws a `Table` widget with rows per job (id, name, runtime, status, phase, attempt, elapsed); refreshes every 250ms from `Arc<Mutex<ServerState>>`; `q` / `Ctrl+C` exits
- Graceful shutdown: Ctrl+C → `CancellationToken::cancel()` → all running job tasks call teardown → dispatch loop exits → TUI thread joins → process exits 0
- `examples/server.toml` with documented fields and inline comments
- `cargo test --workspace` all green; existing single-job tests unaffected

Consumes from S01:
- `dispatch_loop()`, `ServerState`, `CancellationToken`

Consumes from S02:
- `DirectoryWatcher`, HTTP router, `ServerConfig`
