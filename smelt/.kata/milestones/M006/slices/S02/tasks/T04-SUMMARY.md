---
id: T04
parent: S02
milestone: M006
provides:
  - "build_router(state) → axum::Router with 4 HTTP routes for job management"
  - "JobStateResponse — JSON-serialisable snapshot of QueuedJob state"
  - "POST /api/v1/jobs — TOML body ingress with parse+validate, returns job_id"
  - "GET /api/v1/jobs — list all jobs as JSON array"
  - "GET /api/v1/jobs/:id — single job lookup or 404"
  - "DELETE /api/v1/jobs/:id — cancel queued job (200) or conflict if running (409)"
key_files:
  - crates/smelt-cli/src/serve/http_api.rs
  - crates/smelt-cli/src/serve/tests.rs
  - crates/smelt-cli/src/serve/mod.rs
  - Cargo.toml
  - crates/smelt-cli/Cargo.toml
key_decisions:
  - "tempfile persistence via std::mem::forget(TempPath): POST handler writes TOML body to NamedTempFile, converts to TempPath, then forgets it so the file stays on disk for dispatch_loop to read — avoids premature cleanup while keeping temp semantics"
  - "axum path syntax uses {id} (0.8 style) not :id — matches axum 0.8 API"
patterns_established:
  - "HTTP test helper start_test_server(): binds OS-assigned port via TcpListener::bind(127.0.0.1:0), spawns axum::serve in tokio::spawn, returns base URL — all HTTP tests share this pattern"
  - "Status string serialization via match on JobStatus enum in From<&QueuedJob> impl — keeps serde_json output human-readable (queued, running, etc.) rather than relying on serde derive"
observability_surfaces:
  - "GET /api/v1/jobs — full job state array with id, manifest_name, status, attempt, queued_at_secs, started_at_secs, elapsed_secs"
  - "GET /api/v1/jobs/:id — single job inspection"
  - "422 responses include parse/validation error text in body"
duration: 1 step
verification_result: passed
completed_at: 2026-03-23
blocker_discovered: false
---

# T04: HTTP API (axum) with JobStateResponse and full route integration tests

**Implemented axum HTTP API with 4 routes (POST/GET/GET-by-id/DELETE) for job ingestion and state inspection, plus 6 integration tests — all passing.**

## What Happened

Added `axum` 0.8 and `serde_json` to workspace dependencies, plus `reqwest` as a dev dependency for HTTP integration tests. Implemented `http_api.rs` with `JobStateResponse` (JSON-serialisable job state snapshot) and `build_router()` providing 4 routes:

- **POST /api/v1/jobs**: Accepts raw TOML body, parses via `JobManifest::from_str()`, validates via `validate()`, writes to a `NamedTempFile` (persisted via `std::mem::forget`), enqueues via `ServerState::enqueue()`, returns `{"job_id": "..."}`. Returns 422 on parse or validation failure.
- **GET /api/v1/jobs**: Returns JSON array of all `JobStateResponse` entries.
- **GET /api/v1/jobs/:id**: Returns single job or 404.
- **DELETE /api/v1/jobs/:id**: Cancels queued jobs (200), returns 409 Conflict for running/dispatching jobs, 404 for non-existent or terminal.

Replaced the 6 ignored test stubs with real async integration tests using `start_test_server()` helper that binds an OS-assigned port. Also moved `tempfile` from dev-dependencies to production dependencies (needed by POST handler).

## Verification

- `cargo test -p smelt-cli "serve::tests::test_http"` → 6/6 passed
- `cargo test -p smelt-cli serve::tests` → 14/14 passed (all T01–T04 tests)
- `cargo test --workspace` → all test suites green, zero failures
- `cargo build -p smelt-cli` → clean (no errors)

### Slice-level verification (all pass — this is the final task):
- `cargo test -p smelt-cli serve::tests` → 14 passed (queue unit tests + dispatch + watcher + HTTP)
- `cargo build -p smelt-cli 2>&1 | grep -E "^error"` → no errors

## Diagnostics

- `curl http://localhost:<port>/api/v1/jobs | jq .` — shows full live state of all jobs
- `curl http://localhost:<port>/api/v1/jobs/<id> | jq .` — single job inspection
- 422 responses include the parse/validation error message in the response body text
- `JobStateResponse` fields: `id`, `manifest_name`, `status` (queued/dispatching/running/retrying/complete/failed), `attempt`, `queued_at_secs`, `started_at_secs`, `elapsed_secs`

## Deviations

- Added 6 tests instead of 5 (plan said 5, but `test_http_delete_running_job` was listed as a 6th test in the slice plan test list). All 6 pass.
- `tempfile` moved from dev-deps to production deps since the POST handler needs it at runtime, not just in tests.

## Known Issues

- `std::mem::forget(TempPath)` leaks temp files on disk. For a production server this would need a cleanup mechanism (e.g. periodic sweep of old temp files). Acceptable for the current integration stage.

## Files Created/Modified

- `crates/smelt-cli/src/serve/http_api.rs` — Full implementation replacing placeholder: `JobStateResponse`, `build_router()`, 4 route handlers
- `crates/smelt-cli/src/serve/tests.rs` — Replaced 6 ignored stubs with real async integration tests + `start_test_server()` helper
- `crates/smelt-cli/src/serve/mod.rs` — Added `pub(crate) use http_api::build_router;` re-export
- `Cargo.toml` — Added `axum = "0.8"` and `serde_json = "1"` to workspace dependencies
- `crates/smelt-cli/Cargo.toml` — Added axum, serde_json, tempfile to deps; reqwest to dev-deps; removed duplicate tempfile from dev-deps
