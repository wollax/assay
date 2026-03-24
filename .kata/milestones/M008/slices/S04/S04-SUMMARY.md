---
id: S04
parent: M008
milestone: M008
provides:
  - dispatch_loop routes to SSH workers when config.workers is non-empty, falls back to local when empty
  - Round-robin worker selection via round_robin_idx on ServerState with probe-based offline skip
  - All-workers-offline re-queue (status reverted to Queued, running_count decremented, worker_host cleared)
  - worker_host: Option<String> on QueuedJob — set at dispatch time, persisted to queue state TOML
  - worker_host exposed in GET /api/v1/jobs (JobStateResponse) and TUI Worker column
  - run_ssh_job_task — full SSH dispatch lifecycle (deliver_manifest → run_remote_job → sync_state_back → complete)
  - select_worker — round-robin probe loop returning first live worker or None
requires:
  - slice: S01
    provides: WorkerConfig, SshClient trait, SubprocessSshClient, MockSshClient, ServerConfig.workers
  - slice: S02
    provides: deliver_manifest(), run_remote_job()
  - slice: S03
    provides: sync_state_back()
affects: []
key_files:
  - crates/smelt-cli/src/serve/dispatch.rs
  - crates/smelt-cli/src/serve/queue.rs
  - crates/smelt-cli/src/serve/types.rs
  - crates/smelt-cli/src/serve/http_api.rs
  - crates/smelt-cli/src/serve/tui.rs
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/ssh.rs
  - crates/smelt-cli/src/commands/serve.rs
  - crates/smelt-cli/src/serve/tests.rs
key_decisions:
  - "D122: dispatch_loop generic over SshClient for testability — production uses SubprocessSshClient, tests use MockSshClient"
  - "D123: round_robin_idx is volatile (not serialized) — resets to 0 on restart"
  - "D124: All-workers-offline re-queues job without incrementing attempt count"
  - "SshClient trait changed from async fn to impl Future + Send for tokio::spawn compatibility"
patterns_established:
  - "SSH dispatch pattern: select_worker probe → set worker_host → spawn run_ssh_job_task"
  - "worker_host plumbing pattern: types.rs field → http_api.rs From impl → tui.rs column"
  - "Integration test pattern: MockSshClient with pre-loaded result queues, spawn dispatch_loop, poll for terminal status"
observability_surfaces:
  - "GET /api/v1/jobs returns worker_host per job (null for local, string for remote)"
  - "TUI Worker column shows host or '-'"
  - ".smelt-queue-state.toml persists worker_host via QueuedJob serde"
  - "tracing::info! on SSH dispatch entry (job_id, worker_host)"
  - "tracing::warn! on probe failure (host, error)"
  - "tracing::info! on re-queue (job_id, 'all workers offline')"
drill_down_paths:
  - .kata/milestones/M008/slices/S04/tasks/T01-SUMMARY.md
  - .kata/milestones/M008/slices/S04/tasks/T02-SUMMARY.md
  - .kata/milestones/M008/slices/S04/tasks/T03-SUMMARY.md
duration: 38min
verification_result: passed
completed_at: 2026-03-24T12:30:00Z
---

# S04: Dispatch routing + round-robin + TUI/API worker field

**Full SSH dispatch wiring: round-robin worker selection with probe-based failover, worker_host visible in API and TUI, 155 workspace tests green**

## What Happened

This slice wired all S01–S03 primitives (SshClient, deliver_manifest, run_remote_job, sync_state_back) into the dispatch_loop, completing the SSH worker pools feature.

**T01** added `worker_host: Option<String>` to `QueuedJob` (with `#[serde(default)]` for backward compat), `JobStateResponse` (API JSON), and the TUI table (Worker column). All construction sites updated to `worker_host: None`.

**T02** was the core wiring task. `dispatch_loop` gained three new parameters: `workers: Vec<WorkerConfig>`, `ssh_client: C` (generic over `SshClient`), and `ssh_timeout_secs: u64`. When workers is non-empty, each dispatched job goes through `select_worker()` which probes workers in round-robin order starting from `round_robin_idx` on `ServerState`. The first responding worker gets the job via `run_ssh_job_task()`, which orchestrates the full lifecycle: deliver_manifest → run_remote_job → sync_state_back → complete. If all workers are offline, the job is re-queued (status reverted to Queued, running_count decremented). When workers is empty, the existing local path is unchanged. The `SshClient` trait was changed from `async fn` to `fn -> impl Future + Send` to satisfy `tokio::spawn`'s `Send` bound. Removed `#[allow(dead_code)]` from `ssh_timeout_secs` on `ServerConfig`.

**T03** added 4 integration tests: round-robin distribution across 2 mock workers (4 jobs alternate), single-worker failover (offline worker skipped), all-workers-offline re-queue, and worker_host TOML persistence round-trip. All use `MockSshClient`.

## Verification

- `cargo test -p smelt-cli --lib -- dispatch::tests` — 4 passed (select_worker round-robin, one offline skip, all offline, requeue)
- `cargo test -p smelt-cli --lib -- serve::tests::test_round_robin_two_workers` — passed
- `cargo test -p smelt-cli --lib -- serve::tests::test_failover_one_offline` — passed
- `cargo test -p smelt-cli --lib -- serve::tests::test_all_workers_offline_requeue` — passed
- `cargo test -p smelt-cli --lib -- serve::tests::test_worker_host_in_queue_state_roundtrip` — passed
- `cargo test -p smelt-cli --lib -- http_api::tests` — 2 passed (worker_host present and null)
- `cargo test -p smelt-cli --lib -- tui::tests::test_tui_render_worker_host` — passed
- `cargo test --workspace` — 155 passed, 0 failed
- `grep -c 'allow(dead_code)' crates/smelt-cli/src/serve/config.rs` — returns 1 (only `retry_backoff_secs`)

## Requirements Advanced

- R027 (SSH worker pools / remote dispatch) — This slice completes the full dispatch pipeline: worker selection, SSH routing, state sync, API/TUI visibility. All automated proof is complete.

## Requirements Validated

- R027 — Automated proof: MockSshClient integration tests prove round-robin distribution, single-worker failover, all-offline re-queue, worker_host API visibility, worker_host TUI rendering, worker_host TOML persistence. Live multi-host proof deferred to UAT.

## New Requirements Surfaced

- none

## Requirements Invalidated or Re-scoped

- none

## Deviations

- SshClient trait signature changed from `async fn` to `fn -> impl Future + Send` (needed for `tokio::spawn` compatibility). Functionally equivalent; all existing callers updated.
- `test_all_workers_offline_requeue` simulates one dispatch cycle directly rather than running the full dispatch_loop — avoids infinite re-queue loop when all workers are permanently offline in test.

## Known Limitations

- Live multi-host SSH dispatch is not tested by automated tests — requires real remote SSH hosts (covered by UAT).
- `round_robin_idx` resets to 0 on daemon restart (D123) — acceptable for worker selection but means post-restart distribution starts from worker 0.
- No exponential backoff on repeated all-workers-offline scenarios — 2s dispatch tick provides natural retry interval (D124 notes this as revisable).

## Follow-ups

- none — M008 milestone is complete pending UAT.

## Files Created/Modified

- `crates/smelt-cli/src/serve/types.rs` — Added `worker_host: Option<String>` to `QueuedJob`
- `crates/smelt-cli/src/serve/http_api.rs` — Added `worker_host` to `JobStateResponse`, `From` impl, test module
- `crates/smelt-cli/src/serve/tui.rs` — Added Worker column, `test_tui_render_worker_host`
- `crates/smelt-cli/src/serve/queue.rs` — Added `round_robin_idx` to `ServerState`, updated constructors
- `crates/smelt-cli/src/serve/dispatch.rs` — Added `run_ssh_job_task`, `select_worker`, SSH dispatch path in `dispatch_loop`
- `crates/smelt-cli/src/serve/config.rs` — Removed `#[allow(dead_code)]` from `ssh_timeout_secs`
- `crates/smelt-cli/src/serve/ssh.rs` — Changed trait to `impl Future + Send`, added `Clone`, made tests `pub(crate)`
- `crates/smelt-cli/src/commands/serve.rs` — Wired workers, SubprocessSshClient, ssh_timeout_secs into dispatch_loop
- `crates/smelt-cli/src/serve/tests.rs` — Added 4 integration tests, updated dispatch_loop calls

## Forward Intelligence

### What the next slice should know
- M008 is complete. All 4 slices (WorkerConfig, manifest delivery, state sync, dispatch routing) are wired and tested.
- The full SSH dispatch pipeline is: `select_worker` (probe round-robin) → `set worker_host` → `spawn run_ssh_job_task` (deliver_manifest → run_remote_job → sync_state_back → complete).

### What's fragile
- `SubprocessSshClient` depends on `ssh`/`scp` binaries being on PATH — no runtime check at startup, fails at first dispatch attempt
- SSH connection timeout (default 5s) is hardcoded in the ssh command args — not configurable per-worker

### Authoritative diagnostics
- `cargo test -p smelt-cli --lib -- dispatch::tests` — fastest check for dispatch routing logic
- `cargo test -p smelt-cli --lib -- serve::tests` — full integration test suite for serve subsystem
- `grep 'dispatching job to worker\|probe failed\|all workers offline' .smelt/serve.log` — runtime SSH dispatch tracing

### What assumptions changed
- SshClient trait needed `impl Future + Send` instead of `async fn` — `tokio::spawn` requires `Send` futures and `async fn in trait` doesn't guarantee `Send`
