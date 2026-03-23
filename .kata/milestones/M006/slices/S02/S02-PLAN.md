# S02: Directory Watch + HTTP API

**Goal:** Implement all foundational `smelt serve` infrastructure: job types, `JobQueue`, `ServerState`, `dispatch_loop`, `DirectoryWatcher`, and an axum HTTP API — delivering two working job ingress paths backed by real concurrent dispatch.

**Demo:** Drop a `.toml` manifest into `queue_dir/` and it dispatches. POST a manifest body to `POST /api/v1/jobs` and get back a `job_id`. `GET /api/v1/jobs/:id` returns live job state JSON. `DELETE /api/v1/jobs/:id` cancels a queued job (409 if already running). Integration tests for all paths pass with `cargo test`.

## Must-Haves

- `JobId`, `JobStatus`, `JobSource`, `QueuedJob`, `ServerState`, `JobQueue` types exist and are unit-tested
- `JobQueue::enqueue/try_dispatch/complete/cancel/retry_eligible` pass FIFO/cap/retry unit tests
- `dispatch_loop` respects `max_concurrent` and dispatches real tokio tasks calling `run_with_cancellation()`
- `run_job_task` wraps `run_with_cancellation()` with the `CancellationToken` adapter (`async { token.cancelled().await; Ok(()) }`)
- `CancellationToken` broadcast teardown: simulating cancel while 2 jobs run causes both to exit cleanly
- `DirectoryWatcher` polls every 2s, moves manifests atomically to `queue_dir/dispatched/<ts>-<name>.toml` before enqueue
- HTTP API: POST /api/v1/jobs, GET /api/v1/jobs, GET /api/v1/jobs/:id, DELETE /api/v1/jobs/:id — all return correct JSON/status codes
- DELETE running job → 409; DELETE queued job → 200
- HTTP POST with invalid TOML or validation failure → 422
- All new tests pass under `cargo test -p smelt-cli` (Docker-dependent tests skip gracefully when daemon unavailable)

## Proof Level

- This slice proves: integration
- Real runtime required: yes (Docker for dispatch integration tests; skipped gracefully via D024)
- Human/UAT required: no

## Verification

```
# Unit tests (no Docker required)
cargo test -p smelt-cli serve::types
cargo test -p smelt-cli serve::queue

# Integration tests (Docker skip pattern applies)
cargo test -p smelt-cli serve::tests
cargo test -p smelt-cli -- --nocapture 2>&1 | grep -E "SKIP|ok|FAILED"

# Compile check — no new warnings
cargo build -p smelt-cli 2>&1 | grep -E "^error"
```

Test file: `crates/smelt-cli/src/serve/tests.rs` — covers:
- `test_queue_fifo_order` — enqueue 3, dequeue in order
- `test_queue_max_concurrent` — cap of 1 blocks second job until first completes
- `test_queue_cancel_queued` — cancel queued job returns true; cancel running returns false
- `test_queue_retry_eligible` — retry state machine: eligible after failure, not after max_attempts
- `test_dispatch_loop_two_jobs_concurrent` — two real tokio tasks dispatch independently (Docker skip)
- `test_cancellation_broadcast` — CancellationToken::cancel() signals both job tasks (no Docker required — uses oneshot cancel futures)
- `test_watcher_picks_up_manifest` — write TOML to TempDir queue_dir, wait 3s, assert job in ServerState (Docker skip for dispatch)
- `test_watcher_moves_to_dispatched` — file moves to `dispatched/` dir before enqueue
- `test_http_post_enqueues_job` — POST valid TOML → 200 + job_id JSON
- `test_http_post_invalid_toml` → 422
- `test_http_get_jobs` — GET /api/v1/jobs returns array with enqueued job
- `test_http_get_job_by_id` — GET /api/v1/jobs/:id returns correct state
- `test_http_delete_queued_job` → 200
- `test_http_delete_running_job` → 409

## Observability / Diagnostics

- Runtime signals: `tracing::info!` on every enqueue, dispatch, complete, retry, and cancel transition; `tracing::warn!` on watcher parse errors; `tracing::error!` on dispatch task failure
- Inspection surfaces: `GET /api/v1/jobs` returns full job state including `status`, `phase`, `attempt`, `elapsed_secs` — sufficient for a future agent to diagnose stuck or failed jobs
- Failure visibility: `JobStateResponse` includes `status` (Queued/Running/Complete/Failed/Retrying), `exit_code`, `elapsed_secs`, `attempt` — a future agent reading the API response can identify which job failed and when
- Redaction constraints: no secrets or credentials appear in job state; manifest content is not echoed in API responses

## Integration Closure

- Upstream surfaces consumed: `run_with_cancellation()` from `crates/smelt-cli/src/commands/run.rs`; `JobManifest::from_str()` + `validate()` from `smelt-core/src/manifest.rs`; `JobMonitor`/`RunState` from `smelt-core/src/monitor.rs`
- New wiring introduced: `serve/` module wired into `smelt-cli/src/lib.rs` as `pub mod serve`; `axum`, `serde_json`, `tokio-util` added as deps; `CancellationToken` broadcast pattern established for S03 to adopt in `smelt serve` entrypoint
- What remains before milestone is truly usable end-to-end: S03 builds the `smelt serve` CLI subcommand, `ServerConfig` TOML parsing, and Ratatui TUI — wiring all S02 components into a single entrypoint with Ctrl+C graceful shutdown

## Tasks

- [x] **T01: Core types, JobQueue, ServerState, and unit tests** `est:1h`
  - Why: All S01 boundary contracts must exist before watcher and HTTP API can be built; this is the foundation the entire slice depends on
  - Files: `crates/smelt-cli/src/serve/mod.rs`, `crates/smelt-cli/src/serve/types.rs`, `crates/smelt-cli/src/serve/queue.rs`, `crates/smelt-cli/src/serve/tests.rs`, `crates/smelt-cli/src/lib.rs`
  - Do: Create `serve/` module directory. Define `JobId` (String newtype), `JobSource` (DirectoryWatch | HttpApi), `JobStatus` (Queued/Dispatching/Running/Retrying/Complete/Failed — all with Serialize+Clone+Debug), `QueuedJob { id, manifest_path, source, attempt, status, queued_at }`, `ServerState { jobs: VecDeque<QueuedJob>, running_count: usize, max_concurrent: usize }` wrapped in `Arc<Mutex<ServerState>>`. Implement `JobQueue` methods: `enqueue()` adds job with Queued status, `try_dispatch()` returns job if running_count < max_concurrent, `complete()` decrements running_count and sets terminal status, `cancel()` removes Queued job (false if Running), `retry_eligible()` checks attempt < max_attempts. Add unit tests covering FIFO order, concurrency cap, cancel semantics, retry state machine. Wire `pub mod serve;` in `lib.rs`.
  - Verify: `cargo test -p smelt-cli serve::queue -- --nocapture` passes all 4 queue unit tests; `cargo build -p smelt-cli` compiles clean
  - Done when: queue unit tests all pass; types compile with no warnings

- [x] **T02: dispatch_loop, run_job_task, and CancellationToken broadcast** `est:1.5h`
  - Why: The dispatch engine is the core of smelt serve; CancellationToken broadcast is the M006 key risk (must prove before S03)
  - Files: `crates/smelt-cli/src/serve/dispatch.rs`, `crates/smelt-cli/src/serve/tests.rs`, `Cargo.toml` (workspace), `crates/smelt-cli/Cargo.toml`
  - Do: Add `tokio-util = { version = "0.7", features = ["rt"] }` and `uuid = { version = "1", features = ["v4"] }` to workspace deps and smelt-cli deps. Implement `run_job_task(manifest_path: PathBuf, job_id: JobId, state: Arc<Mutex<ServerState>>, cancel_token: CancellationToken)` — loads manifest, builds a `cancel` future as `async { token.cancelled().await; Ok(()) }`, calls `run_with_cancellation()` via `RunArgs { manifest: manifest_path, dry_run: false, no_pr: false }`, updates ServerState on completion. Implement `dispatch_loop(state: Arc<Mutex<ServerState>>, cancel_token: CancellationToken)` — tokio::time::interval(2s) tick; call `try_dispatch()`, if Some spawn `tokio::spawn(run_job_task(..., cancel_token.child_token()))`, loop until cancel_token cancelled. Add integration test `test_dispatch_loop_two_jobs_concurrent` (Docker skip); add `test_cancellation_broadcast` using two oneshot cancel futures (no Docker needed) to prove both tasks receive cancel signal.
  - Verify: `cargo test -p smelt-cli serve::tests::test_cancellation_broadcast -- --nocapture` passes; `cargo test -p smelt-cli serve::tests::test_dispatch_loop_two_jobs_concurrent -- --nocapture` passes or prints "SKIP: Docker unavailable"; `cargo build -p smelt-cli` clean
  - Done when: cancellation broadcast test passes without Docker; dispatch loop compiles; CancellationToken dep in lockfile

- [x] **T03: DirectoryWatcher with atomic file-move and integration test** `est:1h`
  - Why: Directory watch is one of the two required job ingress paths (R023/R024)
  - Files: `crates/smelt-cli/src/serve/queue_watcher.rs`, `crates/smelt-cli/src/serve/tests.rs`, `crates/smelt-cli/src/serve/mod.rs`
  - Do: Implement `DirectoryWatcher { queue_dir: PathBuf, state: Arc<Mutex<ServerState>> }` with `async fn watch(&self)` method. Use `tokio::time::interval(Duration::from_secs(2))`. On each tick: `std::fs::read_dir(queue_dir)`, filter `.toml` files, create `dispatched/` subdir via `create_dir_all`, for each file: `std::fs::rename` to `dispatched/<unix_ts_ms>-<filename>`, then parse manifest via `JobManifest::from_str()`, call `state.lock().unwrap().queue.enqueue(manifest_path, JobSource::DirectoryWatch)`. Skip files that fail parsing with `tracing::warn!`. Add `test_watcher_picks_up_manifest` (writes real TOML to TempDir, starts watcher task, sleeps 3s, checks ServerState) and `test_watcher_moves_to_dispatched` (same but checks file moved). Export `DirectoryWatcher` from `serve/mod.rs`.
  - Verify: `cargo test -p smelt-cli serve::tests::test_watcher -- --nocapture` both watcher tests pass (no Docker required for these); `cargo build -p smelt-cli` clean
  - Done when: both watcher integration tests pass; file-move semantics confirmed by test

- [x] **T04: HTTP API (axum) with JobStateResponse and full route integration tests** `est:1.5h`
  - Why: HTTP API is the second required ingress path (R024); closes S02's integration proof
  - Files: `crates/smelt-cli/src/serve/http_api.rs`, `crates/smelt-cli/src/serve/tests.rs`, `crates/smelt-cli/Cargo.toml`, `Cargo.toml` (workspace)
  - Do: Add `axum = "0.8"` and `serde_json = "1"` to workspace deps and smelt-cli deps. Define `JobStateResponse { id, manifest_name, status, phase, attempt, queued_at_secs, elapsed_secs: Option<f64>, exit_code: Option<i32> }` with `#[derive(Serialize)]`. Build axum router: POST `/api/v1/jobs` — accept `text/plain` body, parse via `JobManifest::from_str()`, call `validate()` (422 on error), write to temp file, enqueue, return `{ "job_id": "..." }`; GET `/api/v1/jobs` — return `Vec<JobStateResponse>`; GET `/api/v1/jobs/:id` — return single or 404; DELETE `/api/v1/jobs/:id` — check status: if Queued → cancel + 200, if Running/Dispatching → 409. Pass `Arc<Mutex<ServerState>>` via axum `State` extractor. Integration tests: bind on `127.0.0.1:0`, read back port, send real HTTP via `reqwest` or `tokio::net::TcpStream`. Add `reqwest = { version = "0.12", features = ["json"] }` to smelt-cli dev-deps. Cover all 5 test cases listed in Verification section.
  - Verify: `cargo test -p smelt-cli serve::tests::test_http -- --nocapture` all 5 HTTP tests pass; `cargo build -p smelt-cli` clean; `cargo test --workspace` green
  - Done when: all HTTP integration tests pass; POST/GET/DELETE return correct status codes and JSON shapes; `cargo test --workspace` shows zero failures

## Files Likely Touched

- `crates/smelt-cli/src/serve/mod.rs` (new)
- `crates/smelt-cli/src/serve/types.rs` (new)
- `crates/smelt-cli/src/serve/queue.rs` (new)
- `crates/smelt-cli/src/serve/dispatch.rs` (new)
- `crates/smelt-cli/src/serve/queue_watcher.rs` (new)
- `crates/smelt-cli/src/serve/http_api.rs` (new)
- `crates/smelt-cli/src/serve/tests.rs` (new)
- `crates/smelt-cli/src/lib.rs` (add `pub mod serve`)
- `crates/smelt-cli/Cargo.toml` (add axum, serde_json, tokio-util, uuid)
- `Cargo.toml` (add workspace deps: axum, serde_json, tokio-util, uuid)
