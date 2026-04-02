---
estimated_steps: 7
estimated_files: 5
---

# T04: HTTP API (axum) with JobStateResponse and full route integration tests

**Slice:** S02 — Directory Watch + HTTP API
**Milestone:** M006

## Description

Implements the second job ingress path and live job state inspection surface: an axum HTTP API with 4 routes. POST accepts raw TOML body, parses it, validates it, writes to a temp file (to satisfy `run_with_cancellation()`'s `PathBuf` requirement), and enqueues. GET routes return job state JSON. DELETE cancels a queued job or returns 409 if running. All tests use `TcpListener::bind("127.0.0.1:0")` for OS-assigned ports. This task closes the S02 integration proof — with T01–T04 complete, both ingress paths work and job state is inspectable via API.

## Steps

1. Add `axum = "0.8"` and `serde_json = "1"` to workspace `Cargo.toml` `[workspace.dependencies]`. Add to `smelt-cli/Cargo.toml` deps: `axum.workspace = true` and `serde_json.workspace = true`. Add to `smelt-cli/Cargo.toml` dev-deps: `reqwest = { version = "0.12", features = ["json", "blocking"] }`. Run `cargo build -p smelt-cli` to verify version resolution.
2. Define `JobStateResponse` in `serve/http_api.rs`: `#[derive(Serialize)] pub(crate) struct JobStateResponse { id: String, manifest_name: String, status: String, attempt: u32, queued_at_secs: u64, started_at_secs: Option<u64>, elapsed_secs: Option<f64>, exit_code: Option<i32> }`. Add a `From<&QueuedJob> for JobStateResponse` impl computing elapsed from `started_at.map(|t| t.elapsed().as_secs_f64())`.
3. Implement `pub(crate) fn build_router(state: Arc<Mutex<ServerState>>) -> axum::Router`. Routes: `POST /api/v1/jobs` — extract `String` body, parse via `JobManifest::from_str(&body, Path::new("http-post"))`, call `validate()` (return 422 with error text on failure), write body to `NamedTempFile` (use `tempfile` crate, already workspace dep), enqueue via `state.lock().unwrap().enqueue(temp_path, JobSource::HttpApi)`, return `Json({ "job_id": id.to_string() })`; `GET /api/v1/jobs` — return `Json(Vec<JobStateResponse>)`; `GET /api/v1/jobs/:id` — find by id or return 404 via `StatusCode::NOT_FOUND`; `DELETE /api/v1/jobs/:id` — check status: if Queued → cancel → 200; if Running/Dispatching → return `StatusCode::CONFLICT` (409).
4. In `serve/tests.rs`, implement 5 HTTP integration tests using `axum::serve` in a `tokio::spawn`. Each test: bind `TcpListener::bind("127.0.0.1:0").await?`, build router, spawn `axum::serve(listener, router)`, send HTTP via `reqwest::Client`. Tests: `test_http_post_enqueues_job` (valid TOML → 200 + job_id), `test_http_post_invalid_toml` (garbage body → 422), `test_http_get_jobs` (after POST → GET returns array with 1 item), `test_http_get_job_by_id` (after POST → GET /:id returns correct id), `test_http_delete_queued_job` (after POST → DELETE → 200), `test_http_delete_running_job` (manually set job to Running in state → DELETE → 409).
5. Export `build_router` in `serve/mod.rs` via `pub(crate) use http_api::build_router;`.
6. Run `cargo test -p smelt-cli serve::tests::test_http -- --nocapture`. Fix all errors.
7. Run `cargo test --workspace` and confirm zero failures; fix any regressions.

## Must-Haves

- [ ] `axum` and `serde_json` added to workspace deps; `cargo build -p smelt-cli` resolves without version conflicts
- [ ] POST `/api/v1/jobs` with valid TOML → 200 + JSON `{ "job_id": "..." }`
- [ ] POST with invalid TOML → 422 (not 500)
- [ ] POST with valid TOML that fails `validate()` → 422
- [ ] GET `/api/v1/jobs` returns JSON array with correct job count
- [ ] GET `/api/v1/jobs/:id` returns single job or 404
- [ ] DELETE Queued job → 200; DELETE Running/Dispatching job → 409
- [ ] All 5 HTTP tests in `tests.rs` pass
- [ ] `cargo test --workspace` green (zero failures)

## Verification

- `cargo test -p smelt-cli serve::tests::test_http -- --nocapture` → 5/5 pass (or 6/6 if delete tests are separate)
- `cargo test --workspace` → zero failures
- `cargo build -p smelt-cli` → clean

## Observability Impact

- Signals added/changed: `GET /api/v1/jobs` is the primary runtime inspection surface for `smelt serve` — a future agent can poll this endpoint to observe all job states, retry counts, and elapsed times without reading log files
- How a future agent inspects this: `curl http://localhost:<port>/api/v1/jobs | jq .` shows full live state; individual `GET /api/v1/jobs/:id` scopes to one job
- Failure state exposed: `JobStateResponse.status` distinguishes Queued/Running/Retrying/Complete/Failed; `attempt` shows retry count; `exit_code` shows terminal exit status — all failure signals are exposed via the API

## Inputs

- `crates/smelt-cli/src/serve/queue.rs` (T01) — `ServerState`, `JobStatus`, `QueuedJob`, `JobSource`
- `smelt-core` manifest — `JobManifest::from_str()` + `validate()` (confirmed in research)
- `tempfile` — already a workspace dep (D080); add to smelt-cli production deps if not already present

## Expected Output

- `crates/smelt-cli/src/serve/http_api.rs` — `JobStateResponse`, `build_router` with 4 routes
- `crates/smelt-cli/src/serve/tests.rs` — 5+ HTTP integration tests all passing
- `crates/smelt-cli/src/serve/mod.rs` — `build_router` exported
- `Cargo.toml` — `axum` and `serde_json` workspace deps
- `crates/smelt-cli/Cargo.toml` — `axum`, `serde_json`, `tempfile` production deps; `reqwest` dev dep
