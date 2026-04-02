---
id: T02
parent: S04
milestone: M008
provides:
  - select_worker() — round-robin worker selection with probe-based offline skip
  - run_ssh_job_task() — full SSH dispatch lifecycle (deliver → run → sync → complete)
  - dispatch_loop now routes to SSH workers when workers vec is non-empty
  - All-workers-offline re-queue path (status reverted to Queued, running_count decremented)
  - worker_host set on QueuedJob before SSH dispatch
key_files:
  - crates/smelt-cli/src/serve/dispatch.rs
  - crates/smelt-cli/src/serve/queue.rs
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/ssh.rs
  - crates/smelt-cli/src/commands/serve.rs
key_decisions:
  - "SshClient trait changed from async fn to impl Future + Send — required for tokio::spawn of SSH job tasks"
  - "dispatch_loop generic over C: SshClient + Clone + Send + Sync + 'static — Clone needed for spawning per-job tasks"
  - "Re-queue path directly reverts job status in-place rather than calling complete() — avoids consuming an attempt on infrastructure failure"
  - "round_robin_idx is volatile (not serialized) — resets to 0 on restart, which is acceptable for worker selection"
patterns_established:
  - "SSH dispatch pattern: select_worker probe → set worker_host → spawn run_ssh_job_task"
  - "SshClient trait uses impl Future + Send return types (not async fn) for tokio::spawn compatibility"
observability_surfaces:
  - "tracing::info! on SSH dispatch (job_id, worker_host, manifest)"
  - "tracing::warn! on probe failure (host, error)"
  - "tracing::info! on re-queue (job_id, 'all workers offline')"
duration: 20min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T02: SSH dispatch routing with round-robin and offline failover in dispatch_loop

**Added `select_worker` round-robin with probe-based skip, `run_ssh_job_task` full SSH lifecycle, and wired `dispatch_loop` to route jobs to SSH workers or fall back to local execution**

## What Happened

Extended `dispatch_loop` to accept a workers vec, ssh_client, and ssh_timeout_secs. When workers is non-empty, each dispatched job goes through `select_worker` which probes workers in round-robin order starting from `round_robin_idx` on `ServerState`. The first responding worker gets the job via `run_ssh_job_task`, which calls `deliver_manifest` → `run_remote_job` → `sync_state_back` → `complete`. If all workers are offline, the job is re-queued (status reverted to Queued, running_count decremented, worker_host cleared). When workers is empty, the existing local `run_job_task` path is used unchanged.

Changed `SshClient` trait methods from `async fn` to `fn -> impl Future + Send` to satisfy `tokio::spawn`'s `Send` bound. Added `Clone` derive to `SubprocessSshClient` and `MockSshClient`. Made `ssh::tests` module `pub(crate)` so `MockSshClient` is accessible from dispatch tests.

Added `round_robin_idx: usize` to `ServerState` (volatile, initialized to 0 in all constructors). Removed `#[allow(dead_code)]` from `ssh_timeout_secs` in config.rs. Updated `serve.rs` to pass `config.workers`, `SubprocessSshClient`, and `config.ssh_timeout_secs` to `dispatch_loop`.

## Verification

- `cargo test -p smelt-cli --lib -- dispatch::tests` — 4 tests pass:
  - `test_select_worker_all_online_round_robin` ✓
  - `test_select_worker_one_offline_skip` ✓
  - `test_select_worker_all_offline` ✓
  - `test_requeue_all_workers_offline` ✓
- `cargo test --workspace` — 155 tests pass, 0 failures
- `grep -c 'allow(dead_code)' crates/smelt-cli/src/serve/config.rs` → 1 (only `retry_backoff_secs`)

### Slice-level checks status:
- `cargo test -p smelt-cli --lib -- dispatch::tests` ✓ (4 pass)
- `cargo test --workspace` ✓ (155 pass)
- `grep allow(dead_code)` ✓ (returns 1)
- `serve::tests::test_round_robin` — not yet created (T03 integration test)
- `serve::tests::test_worker_host_api` — not yet created (T03)
- `tui::tests::test_tui_render_worker_host` — already passes (T01)

## Diagnostics

- SSH dispatch entry: grep for "dispatching job to worker" (job_id, worker_host)
- Probe failure: grep for "probe failed for worker" (host, error)
- Re-queue: grep for "all workers offline — re-queueing job" (job_id)
- worker_host on QueuedJob: None for local, Some(host) for remote — visible in API and TUI

## Deviations

- SshClient trait changed from `#[allow(async_fn_in_trait)] async fn` to explicit `fn -> impl Future + Send` — required because `tokio::spawn` needs `Send` futures and `async fn in trait` doesn't guarantee `Send`. This is a signature-compatible change.
- Re-queue test uses direct function calls instead of running the full dispatch_loop to avoid needing unbounded mock probe results for repeated tick cycles. The logic exercised is identical to what dispatch_loop executes.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/dispatch.rs` — Added `run_ssh_job_task`, `select_worker`, modified `dispatch_loop` to accept workers/ssh_client/timeout, added test module with 4 tests
- `crates/smelt-cli/src/serve/queue.rs` — Added `round_robin_idx: usize` to `ServerState`, initialized in all constructors
- `crates/smelt-cli/src/serve/config.rs` — Removed `#[allow(dead_code)]` from `ssh_timeout_secs`
- `crates/smelt-cli/src/serve/ssh.rs` — Changed trait methods to `impl Future + Send`, added `Clone` to `SubprocessSshClient` and `MockSshClient`, made tests module `pub(crate)`
- `crates/smelt-cli/src/commands/serve.rs` — Wired `config.workers`, `SubprocessSshClient`, `config.ssh_timeout_secs` into `dispatch_loop` call
- `crates/smelt-cli/src/serve/tests.rs` — Updated `dispatch_loop` call to pass new args
