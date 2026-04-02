# S02: Directory Watch + HTTP API — UAT

**Milestone:** M006
**Written:** 2026-03-23

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: All S02 deliverables are internal infrastructure (types, queue engine, watcher, HTTP API) — no user-facing CLI subcommand exists yet (that's S03). All code paths are exercised by 14 automated integration tests with real HTTP, real file I/O, and real tokio tasks. No live runtime or human experience testing is needed at this stage.

## Preconditions

- Rust toolchain installed (`cargo` available)
- Docker daemon running (for dispatch integration test; test skips gracefully if unavailable)
- No port conflicts on `127.0.0.1` ephemeral ports (HTTP tests bind to port 0)

## Smoke Test

```bash
cargo test -p smelt-cli serve -- --nocapture
# Expected: 14 passed; 0 failed; 0 ignored
```

## Test Cases

### 1. Queue FIFO and concurrency cap

1. Run `cargo test -p smelt-cli serve::tests::test_queue_fifo_order -- --nocapture`
2. Run `cargo test -p smelt-cli serve::tests::test_queue_max_concurrent -- --nocapture`
3. **Expected:** Both pass — jobs dequeue in insertion order; second job blocks until first completes when max_concurrent=1

### 2. Cancel and retry semantics

1. Run `cargo test -p smelt-cli serve::tests::test_queue_cancel_queued -- --nocapture`
2. Run `cargo test -p smelt-cli serve::tests::test_queue_retry_eligible -- --nocapture`
3. **Expected:** Cancel returns true for queued jobs, false for running. Retry eligible when status=Retrying and attempt < max_attempts.

### 3. CancellationToken broadcast

1. Run `cargo test -p smelt-cli serve::tests::test_cancellation_broadcast -- --nocapture`
2. **Expected:** Pass — parent token cancel triggers both child tasks to exit (no Docker required)

### 4. Directory watcher picks up and moves manifests

1. Run `cargo test -p smelt-cli serve::tests::test_watcher_picks_up_manifest -- --nocapture`
2. Run `cargo test -p smelt-cli serve::tests::test_watcher_moves_to_dispatched -- --nocapture`
3. **Expected:** Both pass — TOML file in queue_dir is enqueued and moved to dispatched/ subdirectory

### 5. HTTP API routes

1. Run `cargo test -p smelt-cli "serve::tests::test_http" -- --nocapture`
2. **Expected:** All 6 pass — POST valid TOML (200 + job_id), POST invalid (422), GET list, GET by id, DELETE queued (200), DELETE running (409)

### 6. Workspace regression

1. Run `cargo test --workspace`
2. **Expected:** All tests pass (155+ total), 0 failures — `smelt run` single-job path unchanged

## Edge Cases

### Invalid TOML body via HTTP POST

1. POST `this is not valid toml` to `/api/v1/jobs`
2. **Expected:** 422 Unprocessable Entity with parse error in response body

### DELETE non-existent job

1. DELETE `/api/v1/jobs/job-99999`
2. **Expected:** 404 Not Found

### Watcher with unparseable file

1. Write a file with `.toml` extension but invalid content to queue_dir
2. **Expected:** File is moved to dispatched/ but not enqueued; `tracing::warn!` emitted

## Failure Signals

- Any `cargo test -p smelt-cli serve` failure indicates broken queue/dispatch/watcher/HTTP logic
- `cargo test --workspace` regressions indicate S02 changes broke existing `smelt run` path
- Dead code warnings for `dispatch_loop`, `run_job_task`, `DirectoryWatcher`, `build_router` are expected (wired in S03)
- `error[E0...]` in `cargo build -p smelt-cli` indicates compile failure

## Requirements Proved By This UAT

- R024 (partial) — HTTP API shape and behavior proven by 6 integration tests (POST/GET/DELETE with correct status codes and JSON). Full validation requires S03 wiring into `smelt serve` entrypoint.
- R023 (partial) — Concurrent dispatch engine with CancellationToken broadcast proven by integration tests. Full validation requires S03 end-to-end with Ratatui TUI and Ctrl+C shutdown.

## Not Proven By This UAT

- `smelt serve` CLI subcommand does not exist yet — S03 builds the entrypoint
- No live HTTP server test (binding to a real port for manual curl testing) — all HTTP tests are in-process
- No Ratatui TUI rendering — S03 deliverable
- No Ctrl+C graceful shutdown of a running `smelt serve` process — S03 deliverable
- No ServerConfig TOML parsing — S03 deliverable
- No real Docker job dispatch through the full watcher→dispatch→run path (tests exercise each component independently; end-to-end requires S03)

## Notes for Tester

- Docker daemon availability is only needed for `test_dispatch_loop_two_jobs_concurrent`; all other tests run without Docker
- HTTP integration tests use OS-assigned ports — no conflicts possible
- Dead code warnings are intentional and expected until S03 wires the components
- The `VALID_MANIFEST_TOML` constant in tests.rs is the canonical minimal valid manifest; if `JobManifest` schema changes upstream, this constant must be updated
