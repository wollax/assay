---
estimated_steps: 6
estimated_files: 4
---

# T02: dispatch_loop, run_job_task, and CancellationToken broadcast

**Slice:** S02 — Directory Watch + HTTP API
**Milestone:** M006

## Description

This task implements the dispatch engine that drives all concurrent job execution. It also proves the M006 key risk: CancellationToken broadcast to N concurrent jobs. The `run_job_task` function bridges `run_with_cancellation()`'s generic `Future<Output=std::io::Result<()>>` cancel parameter to `CancellationToken::cancelled()` using the one-liner adapter `async { token.cancelled().await; Ok(()) }`. The `dispatch_loop` drives the queue and spawns tokio tasks. Two integration tests prove correctness: one with mock cancel futures (no Docker) and one with real dispatch (Docker skip).

## Steps

1. Add deps to workspace `Cargo.toml`: `tokio-util = { version = "0.7", features = ["rt"] }` and `uuid = { version = "1", features = ["v4"] }`. Add to `smelt-cli/Cargo.toml` deps: `tokio-util.workspace = true` and `uuid.workspace = true`.
2. Write `serve/dispatch.rs`. Implement `pub(crate) async fn run_job_task(manifest_path: PathBuf, job_id: JobId, state: Arc<Mutex<ServerState>>, cancel_token: CancellationToken, max_attempts: u32)`. Body: update state to Running (status=Running, started_at=Instant::now()); build `RunArgs { manifest: manifest_path.clone(), dry_run: false, no_pr: false }`; build cancel future `let cancel_fut = { let t = cancel_token.clone(); async move { t.cancelled().await; Ok(()) } }`; call `run_with_cancellation(&args, cancel_fut).await`; on result update state via `state.lock().unwrap().complete(job_id, success, attempt, max_attempts)`. Add `tracing::info!` on start/complete/retry transitions.
3. Implement `pub(crate) async fn dispatch_loop(state: Arc<Mutex<ServerState>>, cancel_token: CancellationToken, max_attempts: u32)`. Use `let mut interval = tokio::time::interval(Duration::from_secs(2))`. Loop: `tokio::select! { _ = interval.tick() => { ... }, _ = cancel_token.cancelled() => break }`. On tick: `if let Some(job) = state.lock().unwrap().try_dispatch() { let state2 = Arc::clone(&state); let child = cancel_token.child_token(); tokio::spawn(run_job_task(job.manifest_path, job.id, state2, child, max_attempts)); }`.
4. In `serve/tests.rs` fill in `test_cancellation_broadcast`: create two dummy cancel futures via `tokio::sync::oneshot`; start two mock jobs (or use `dispatch_loop` with a test state and no real manifest); call `cancel_token.cancel()`; assert both oneshot receivers fire within 500ms. This test must NOT require Docker.
5. In `serve/tests.rs` fill in `test_dispatch_loop_two_jobs_concurrent`: skip if Docker unavailable (check `bollard::Docker::connect_with_local_defaults().await`); create two real manifest TOMLs in TempDir; start dispatch_loop with max_concurrent=2; wait for both to complete; check both jobs reach Complete status in ServerState.
6. Run `cargo test -p smelt-cli serve::tests::test_cancellation_broadcast -- --nocapture` (must pass without Docker). Fix compilation errors.

## Must-Haves

- [ ] `tokio-util` and `uuid` added to workspace + smelt-cli deps; `cargo build -p smelt-cli` succeeds
- [ ] `run_job_task` uses the `CancellationToken` adapter: `async { token.cancelled().await; Ok(()) }` passed to `run_with_cancellation()`
- [ ] `dispatch_loop` respects `max_concurrent` and breaks cleanly when `cancel_token.cancelled()` fires
- [ ] `test_cancellation_broadcast` passes without Docker — proves token is cloned and broadcast reaches all tasks
- [ ] `test_dispatch_loop_two_jobs_concurrent` passes (or prints "SKIP: Docker unavailable" and returns without panicking)
- [ ] `tracing::info!` logged on job start, complete, and retry in `run_job_task`

## Verification

- `cargo test -p smelt-cli serve::tests::test_cancellation_broadcast -- --nocapture` → PASS (no Docker)
- `cargo test -p smelt-cli serve::tests::test_dispatch_loop_two_jobs_concurrent -- --nocapture` → PASS or SKIP
- `cargo build -p smelt-cli` → clean

## Observability Impact

- Signals added/changed: `tracing::info!` at dispatch start, job complete, retry decision, and cancel received — key lifecycle transitions are visible in SMELT_LOG output
- How a future agent inspects this: `SMELT_LOG=info smelt serve` will show all dispatch transitions; `GET /api/v1/jobs` (T04) will show live status
- Failure state exposed: `run_job_task` updates ServerState with Failed/Retrying on non-zero exit code; attempt count increments are visible in the state

## Inputs

- `crates/smelt-cli/src/serve/queue.rs` (T01) — `ServerState`, `JobQueue` methods, `JobId`, `QueuedJob`
- `crates/smelt-cli/src/commands/run.rs` — `run_with_cancellation()` signature: `pub async fn run_with_cancellation<F>(args: &RunArgs, cancel: F) -> Result<i32> where F: Future<Output = std::io::Result<()>> + Send`

## Expected Output

- `crates/smelt-cli/src/serve/dispatch.rs` — `run_job_task` and `dispatch_loop` with CancellationToken integration
- `crates/smelt-cli/src/serve/tests.rs` — cancellation broadcast test passing; dispatch integration test passing/skipping
- `Cargo.toml` — `tokio-util` and `uuid` workspace deps
- `crates/smelt-cli/Cargo.toml` — `tokio-util` and `uuid` added
