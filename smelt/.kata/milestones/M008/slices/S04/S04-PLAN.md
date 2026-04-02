# S04: Dispatch routing + round-robin + TUI/API worker field

**Goal:** `dispatch_loop` routes to SSH workers when `config.workers` is non-empty (falls back to local when empty); round-robin index tracked in `ServerState`; offline worker re-queues job; `worker_host` visible in `GET /api/v1/jobs` and TUI; end-to-end integration test with 2 mock workers confirms round-robin and failover.
**Demo:** Submit 4 jobs to a `smelt serve` with 2 configured workers — jobs alternate between workers via round-robin; if one worker is offline, its jobs route to the surviving worker; `GET /api/v1/jobs` and TUI both show `worker_host`.

## Must-Haves

- `dispatch_loop` uses SSH dispatch (deliver_manifest → run_remote_job → sync_state_back) when `config.workers` is non-empty
- `dispatch_loop` uses local `run_with_cancellation` when `config.workers` is empty (zero regression)
- Round-robin worker index in `ServerState`, wrapping modulo `workers.len()`
- Offline worker (probe fails) → skip worker, try next; if all workers offline → re-queue job (status back to Queued)
- `worker_host: Option<String>` field on `QueuedJob` — set at dispatch time, persisted to queue state TOML
- `worker_host` exposed in `JobStateResponse` JSON via `GET /api/v1/jobs`
- TUI shows worker host column
- `#[allow(dead_code)]` removed from `ssh_timeout_secs` on `ServerConfig`
- `cargo test --workspace` all green

## Proof Level

- This slice proves: integration (mock SSH + real queue state machine + real HTTP API + real TUI render)
- Real runtime required: no (MockSshClient for SSH, real ServerState + axum for API)
- Human/UAT required: yes — real multi-host SSH dispatch deferred to UAT

## Verification

- `cargo test -p smelt-cli --lib -- dispatch::tests` — all new dispatch routing tests pass
- `cargo test -p smelt-cli --lib -- serve::tests::test_round_robin` — round-robin + failover integration test passes
- `cargo test -p smelt-cli --lib -- serve::tests::test_worker_host_api` — worker_host in API response test passes
- `cargo test -p smelt-cli --lib -- tui::tests::test_tui_render_worker_host` — TUI renders worker_host column without panic
- `cargo test --workspace` — all tests pass, 0 failures
- `grep -c 'allow(dead_code)' crates/smelt-cli/src/serve/config.rs` returns 1 (only `retry_backoff_secs` remains)

## Observability / Diagnostics

- Runtime signals: `tracing::info!` on SSH dispatch entry (job_id, worker_host); `tracing::warn!` on worker probe failure (host, error); `tracing::info!` on re-queue due to all-workers-offline
- Inspection surfaces: `GET /api/v1/jobs` returns `worker_host` per job; TUI shows Worker column; queue state TOML persists `worker_host`
- Failure visibility: probe failure logged with host + error detail; re-queued job retains original attempt count; `worker_host` is `None` for locally-dispatched jobs (distinguishes local vs remote)
- Redaction constraints: SSH key paths appear only in DEBUG logs (D112)

## Integration Closure

- Upstream surfaces consumed: `SshClient` trait + `MockSshClient` (S01/S02), `deliver_manifest()` (S02), `run_remote_job()` (S02), `sync_state_back()` (S03), `WorkerConfig` + `ServerConfig` (S01)
- New wiring introduced in this slice: `dispatch_loop` gains SSH dispatch path; `ServerState` gains round-robin index and `worker_host` on `QueuedJob`; `JobStateResponse` gains `worker_host`; TUI gains Worker column
- What remains before the milestone is truly usable end-to-end: UAT with real SSH hosts (manual testing)

## Tasks

- [x] **T01: Add worker_host to QueuedJob, JobStateResponse, and TUI** `est:30m`
  - Why: The data model must carry worker_host before dispatch routing can set it. This task adds the field everywhere it's consumed — queue state, API, TUI — so T02 only needs to set the field at dispatch time.
  - Files: `crates/smelt-cli/src/serve/types.rs`, `crates/smelt-cli/src/serve/http_api.rs`, `crates/smelt-cli/src/serve/tui.rs`, `crates/smelt-cli/src/serve/queue.rs`
  - Do: Add `worker_host: Option<String>` to `QueuedJob` (with `#[serde(default)]`); add `worker_host` to `JobStateResponse`; add Worker column to TUI table; update any test helpers that construct `QueuedJob` to include `worker_host: None`; add `test_tui_render_worker_host` and `test_worker_host_api` tests.
  - Verify: `cargo test --workspace` passes; TUI render test doesn't panic; API response includes `worker_host` field.
  - Done when: `worker_host` field flows from `QueuedJob` through API JSON and TUI render — all tests green.

- [x] **T02: Implement SSH dispatch routing with round-robin and offline failover in dispatch_loop** `est:45m`
  - Why: This is the core wiring — makes `dispatch_loop` route jobs to SSH workers when configured. Adds round-robin index to `ServerState`, probe-based offline detection, and re-queue on all-workers-offline.
  - Files: `crates/smelt-cli/src/serve/dispatch.rs`, `crates/smelt-cli/src/serve/queue.rs`, `crates/smelt-cli/src/serve/config.rs`, `crates/smelt-cli/src/serve/mod.rs`, `crates/smelt-cli/src/commands/serve.rs`
  - Do: Add `round_robin_idx: usize` to `ServerState`; extract `run_ssh_job_task` function; modify `dispatch_loop` to accept workers + ssh_client + ssh_timeout; implement round-robin selection with probe → skip → try next logic; on all-offline re-queue the job; set `worker_host` on the `QueuedJob` at dispatch time; remove `#[allow(dead_code)]` from `ssh_timeout_secs`; wire config.workers into `dispatch_loop` call in `serve.rs`.
  - Verify: `cargo test --workspace` passes; new dispatch routing unit tests pass.
  - Done when: `dispatch_loop` routes to SSH workers when `config.workers` is non-empty; local dispatch unchanged when empty; round-robin and failover proven by tests.

- [x] **T03: End-to-end integration tests for round-robin, failover, and worker_host visibility** `est:30m`
  - Why: Proves the full pipeline: enqueue → dispatch → SSH mock → worker_host visible in API + queue state. Validates round-robin distribution and offline-worker failover with 2 mock workers.
  - Files: `crates/smelt-cli/src/serve/tests.rs`
  - Do: Write `test_round_robin_two_workers` — 2 mock workers, 4 jobs, assert jobs alternate between workers and `worker_host` is set correctly. Write `test_failover_one_offline` — 2 workers, one returns probe error, assert all jobs go to the surviving worker. Write `test_all_workers_offline_requeue` — both workers offline, assert job returns to Queued status. All tests use MockSshClient.
  - Verify: `cargo test -p smelt-cli --lib -- serve::tests::test_round_robin` + `test_failover` + `test_all_workers_offline` all pass.
  - Done when: Round-robin distribution, single-worker failover, and all-offline re-queue are all proven by tests.

## Files Likely Touched

- `crates/smelt-cli/src/serve/types.rs`
- `crates/smelt-cli/src/serve/queue.rs`
- `crates/smelt-cli/src/serve/dispatch.rs`
- `crates/smelt-cli/src/serve/http_api.rs`
- `crates/smelt-cli/src/serve/tui.rs`
- `crates/smelt-cli/src/serve/config.rs`
- `crates/smelt-cli/src/serve/mod.rs`
- `crates/smelt-cli/src/commands/serve.rs`
- `crates/smelt-cli/src/serve/tests.rs`
