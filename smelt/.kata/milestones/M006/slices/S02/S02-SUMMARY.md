---
id: S02
parent: M006
milestone: M006
provides:
  - "JobId, JobSource, JobStatus, QueuedJob, ServerState — full serve type system with queue state machine"
  - "dispatch_loop + run_job_task — concurrent tokio dispatch engine with CancellationToken broadcast"
  - "DirectoryWatcher — poll-and-move ingress path (queue_dir/ → dispatched/)"
  - "HTTP API via axum — POST /api/v1/jobs, GET /api/v1/jobs, GET /api/v1/jobs/:id, DELETE /api/v1/jobs/:id"
  - "JobStateResponse — JSON-serialisable snapshot for HTTP API responses"
  - "14 tests: 4 queue unit, 2 dispatch/cancellation, 2 watcher integration, 6 HTTP integration"
requires:
  - slice: S01
    provides: "S01 was skipped — S02 absorbed all foundational work (types, queue, dispatch)"
affects:
  - S03
key_files:
  - crates/smelt-cli/src/serve/mod.rs
  - crates/smelt-cli/src/serve/types.rs
  - crates/smelt-cli/src/serve/queue.rs
  - crates/smelt-cli/src/serve/dispatch.rs
  - crates/smelt-cli/src/serve/queue_watcher.rs
  - crates/smelt-cli/src/serve/http_api.rs
  - crates/smelt-cli/src/serve/tests.rs
  - crates/smelt-cli/src/lib.rs
  - crates/smelt-cli/Cargo.toml
  - Cargo.toml
key_decisions:
  - "D098: In-process tokio tasks (not subprocess) for job execution — direct state sharing, D037-compatible"
  - "D099: CancellationToken (tokio-util) for broadcast cancellation — supersedes D037 for multi-job context"
  - "D100: Queue pickup via file-move: manifest → dispatched/<ts>-<name>.toml — atomic, restart-safe"
  - "D101: axum for HTTP API — tokio-native, ergonomic routing"
  - "D103: JobId uses atomic u64 counter for test determinism"
  - "D104: ServerState::complete() sets Retrying in-place (not re-enqueue) — single entry per job"
  - "D105: HTTP POST persists TOML body via std::mem::forget(TempPath) — decouples file lifetime from handler"
patterns_established:
  - "serve/ module structure: mod.rs declares submodules, each concern in its own file (types, queue, dispatch, queue_watcher, http_api, tests)"
  - "CancellationToken adapter: async { token.cancelled().await; Ok(()) } bridges tokio_util to generic Future<Output=io::Result<()>>"
  - "child_token per spawned task: parent cancel propagates to all in-flight jobs atomically"
  - "Inner drain loop in dispatch_loop: dispatch all eligible jobs per tick, not just one per 2s interval"
  - "File-move-before-enqueue (D100): std::fs::rename before ServerState::enqueue() prevents double-pickup"
  - "HTTP test helper start_test_server(): bind 127.0.0.1:0, spawn axum, return base URL — all HTTP tests share this"
observability_surfaces:
  - "GET /api/v1/jobs — full job state array (id, manifest_name, status, attempt, queued_at_secs, started_at_secs, elapsed_secs)"
  - "GET /api/v1/jobs/:id — single job inspection or 404"
  - "422 responses include parse/validation error text in body"
  - "tracing::info! on enqueue, dispatch, complete, retry, cancel transitions"
  - "tracing::warn! on watcher parse/rename errors"
  - "queue_dir/dispatched/ directory as file-level inspection surface"
drill_down_paths:
  - .kata/milestones/M006/slices/S02/tasks/T01-SUMMARY.md
  - .kata/milestones/M006/slices/S02/tasks/T02-SUMMARY.md
  - .kata/milestones/M006/slices/S02/tasks/T03-SUMMARY.md
  - .kata/milestones/M006/slices/S02/tasks/T04-SUMMARY.md
duration: ~4h
verification_result: passed
completed_at: 2026-03-23
---

# S02: Directory Watch + HTTP API

**Full `smelt serve` dispatch infrastructure: JobQueue with concurrency cap and retry state machine, CancellationToken broadcast teardown, directory watcher with atomic file-move ingress, and axum HTTP API with 4 routes — all 14 tests pass, cargo test --workspace green.**

## What Happened

S01 was skipped — S02 absorbed all foundational work that the roadmap originally assigned to S01 (types, queue, dispatch, cancellation). This was a practical decision: the boundary map showed S01 and S02 share the same types and state structures, so building them separately would have required artificial stubs.

**T01** established the `serve/` module from scratch with `JobId` (atomic u64 counter), `JobSource` (DirectoryWatch | HttpApi), `JobStatus` (6-variant enum), `QueuedJob`, and `ServerState` with full queue state machine methods (enqueue, try_dispatch, complete, cancel, retry_eligible). Key design: retry keeps a single VecDeque entry with `status = Retrying` in-place rather than re-enqueuing — avoids duplicate entries.

**T02** implemented `dispatch_loop` (2s interval with inner drain loop to dispatch all eligible jobs per tick) and `run_job_task` (wraps `run_with_cancellation()` with a CancellationToken adapter). The cancellation broadcast test proved parent→child token propagation without Docker. The dispatch loop integration test ran with real tokio tasks.

**T03** delivered `DirectoryWatcher` that polls `queue_dir/` every 2s, atomically moves `.toml` files to `dispatched/<unix_ms>-<name>.toml` before enqueue (D100 — prevents double-pickup), and skips unparseable manifests with `tracing::warn!`. Both integration tests use TempDir + tokio::spawn + sleep(3s).

**T04** added the axum HTTP API with 4 routes: POST `/api/v1/jobs` (TOML body → parse + validate → enqueue → `{"job_id": "..."}`), GET `/api/v1/jobs` (array of all job states), GET `/api/v1/jobs/:id` (single or 404), DELETE `/api/v1/jobs/:id` (cancel queued → 200, running → 409). `JobStateResponse` provides a JSON-serialisable snapshot of job state. All 6 HTTP integration tests use a shared `start_test_server()` helper that binds on `127.0.0.1:0`.

## Verification

```
cargo test -p smelt-cli serve -- --nocapture
  14 passed; 0 failed; 0 ignored

cargo build -p smelt-cli 2>&1 | grep -E "^error"
  (none)

cargo test --workspace
  155+ passed; 0 failed
```

All 14 serve tests pass:
- 4 queue unit tests (FIFO, max_concurrent, cancel, retry)
- 2 dispatch tests (cancellation broadcast, concurrent dispatch)
- 2 watcher integration tests (manifest pickup, file move)
- 6 HTTP integration tests (POST valid, POST invalid, GET list, GET by id, DELETE queued, DELETE running)

Dead code warnings expected for `dispatch_loop`, `run_job_task`, `DirectoryWatcher`, `build_router` — these are wired by S03's `smelt serve` entrypoint.

## Requirements Advanced

- R023 — Parallel dispatch daemon now has working concurrent dispatch engine with CancellationToken broadcast; two ingress paths (directory watch + HTTP) both enqueue and dispatch real jobs
- R024 — HTTP API fully implemented: POST/GET/GET-by-id/DELETE with correct status codes and JSON shapes; integration-tested with real HTTP via reqwest

## Requirements Validated

- None moved to validated — R023 and R024 require S03 (the `smelt serve` entrypoint wiring them together) before full validation

## New Requirements Surfaced

- None

## Requirements Invalidated or Re-scoped

- None

## Deviations

- S01 was skipped entirely — S02 absorbed all S01 work (types, queue, dispatch, cancellation). The S01/S02 boundary in the roadmap was artificial.
- Retry semantics: plan implied re-enqueuing a fresh entry on failure. Actual: `complete()` sets `Retrying` in-place; `try_dispatch()` picks up both `Queued` and `Retrying` jobs (D104).
- Test manifest TOML required discovering the real mandatory schema (job.base_ref, [credentials], [[session]], [merge]) — plan referenced `[[step]]` which doesn't exist.
- 6 HTTP tests instead of the plan's listed 5 — the DELETE running job test was listed in the plan's test inventory but not counted in the "5 test cases" text.

## Known Limitations

- `std::mem::forget(TempPath)` in POST handler leaks temp files on disk. For a long-running daemon this needs periodic cleanup. Acceptable for the current integration stage (D105).
- Dead code warnings for all `pub(crate)` serve functions — expected until S03 wires the `smelt serve` entrypoint.
- No authentication on the HTTP API (trusted local network assumed per R024 notes).
- No CORS or rate limiting — S03 may add tower middleware if needed.

## Follow-ups

- S03 must wire all S02 components into the `smelt serve` CLI subcommand with `ServerConfig` TOML parsing, Ratatui TUI, and Ctrl+C graceful shutdown.
- Temp file cleanup mechanism for POST handler should be considered during S03 or a later maintenance pass.

## Files Created/Modified

- `crates/smelt-cli/src/serve/mod.rs` — module declarations + pub(crate) re-exports
- `crates/smelt-cli/src/serve/types.rs` — JobId, JobSource, JobStatus, QueuedJob types
- `crates/smelt-cli/src/serve/queue.rs` — ServerState with full queue state machine
- `crates/smelt-cli/src/serve/dispatch.rs` — run_job_task + dispatch_loop with CancellationToken broadcast
- `crates/smelt-cli/src/serve/queue_watcher.rs` — DirectoryWatcher with atomic file-move
- `crates/smelt-cli/src/serve/http_api.rs` — axum HTTP API with 4 routes + JobStateResponse
- `crates/smelt-cli/src/serve/tests.rs` — 14 tests (queue, dispatch, watcher, HTTP)
- `crates/smelt-cli/src/lib.rs` — added `pub mod serve;`
- `crates/smelt-cli/Cargo.toml` — added axum, serde_json, serde, tokio-util, uuid, tempfile deps; reqwest dev-dep
- `Cargo.toml` — added axum, serde_json, tokio-util, uuid workspace deps

## Forward Intelligence

### What the next slice should know
- All `serve/` types and functions are `pub(crate)` — S03 imports them directly from the same crate
- `dispatch_loop()` signature: `(state: Arc<Mutex<ServerState>>, cancel_token: CancellationToken, max_attempts: u32)` — S03 passes these from `ServerConfig`
- `DirectoryWatcher::new(queue_dir, state)` then `watcher.watch().await` — runs as a `tokio::spawn` task
- `build_router(state: Arc<Mutex<ServerState>>)` returns an `axum::Router` — S03 binds it to `TcpListener`
- `ServerState::new(max_concurrent)` is the constructor — takes the concurrency cap from config

### What's fragile
- `std::mem::forget(TempPath)` in HTTP POST handler — if the dispatch_loop crashes before reading the temp file, the manifest body is lost; no persistence guarantee
- `dispatch_loop` uses `tokio::time::interval(2s)` — if `try_dispatch()` or `run_job_task` panics, the interval continues but the panicked task is lost silently; S03 should add `JoinHandle` tracking

### Authoritative diagnostics
- `cargo test -p smelt-cli serve` — runs all 14 serve tests; this is the single command that proves S02 correctness
- `GET /api/v1/jobs` — the canonical runtime inspection surface for job state; S03's TUI reads from the same `Arc<Mutex<ServerState>>`

### What assumptions changed
- S01 was assumed to be a separate deliverable — in practice, the types/queue/dispatch were inseparable from watcher/HTTP and S02 absorbed everything
- `JobManifest` schema was stricter than expected: `deny_unknown_fields` + mandatory `base_ref`, `[credentials]`, `[[session]]`, `[merge]` — test manifest construction required all sections
