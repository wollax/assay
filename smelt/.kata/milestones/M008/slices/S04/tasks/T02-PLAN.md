---
estimated_steps: 5
estimated_files: 6
---

# T02: Implement SSH dispatch routing with round-robin and offline failover in dispatch_loop

**Slice:** S04 — Dispatch routing + round-robin + TUI/API worker field
**Milestone:** M008

## Description

Make `dispatch_loop` route jobs to SSH workers when `config.workers` is non-empty. Add `round_robin_idx` to `ServerState` for worker selection. Implement probe → skip → try next logic for offline workers. Re-queue the job when all workers are offline. Set `worker_host` on the `QueuedJob` at dispatch time. Wire `ServerConfig` fields (workers, ssh_timeout_secs) into the dispatch loop call in `serve.rs`.

## Steps

1. Add `round_robin_idx: usize` to `ServerState` (initialized to 0 in `new`, `new_with_persistence`, `load_or_new`). Not serialized — it's volatile state.
2. Create `run_ssh_job_task<C: SshClient>()` in `dispatch.rs` — takes `(manifest_path, job_id, worker, state, ssh_client, timeout_secs, max_attempts)`; calls `deliver_manifest` → `run_remote_job` → `sync_state_back`; calls `state.complete()` with the result. Parse `job_name` from manifest via `JobManifest::from_str()` for `sync_state_back`.
3. Create `select_worker<C: SshClient>()` in `dispatch.rs` — takes `(workers, ssh_client, timeout_secs, round_robin_idx)` → returns `Option<(WorkerConfig, usize)>`. Starts at `round_robin_idx % workers.len()`, probes up to `workers.len()` workers in round-robin order, returns the first that responds successfully (and the new index). Returns `None` if all offline.
4. Modify `dispatch_loop` signature to accept workers config and ssh_client generically: `dispatch_loop<C: SshClient + Send + Sync + 'static>(state, cancel_token, max_attempts, workers: Vec<WorkerConfig>, ssh_client: C, ssh_timeout_secs: u64)`. When `workers.is_empty()`, use existing local `run_job_task` path. When non-empty: call `select_worker` → if `Some(worker)` → set `worker_host` on the job in `ServerState` → spawn `run_ssh_job_task`; if `None` → log warning, revert job status to `Queued`, decrement `running_count`.
5. Remove `#[allow(dead_code)]` from `ssh_timeout_secs` in `config.rs`. Update `serve.rs` to pass `config.workers.clone()`, `SubprocessSshClient`, and `config.ssh_timeout_secs` to `dispatch_loop`. Add unit tests for `select_worker` (all-online round-robin, one-offline skip, all-offline returns None) and for the re-queue path.

## Must-Haves

- [ ] `ServerState` has `round_robin_idx: usize` (non-serialized, volatile)
- [ ] `dispatch_loop` dispatches via SSH when `workers` is non-empty
- [ ] `dispatch_loop` dispatches locally when `workers` is empty (no regression)
- [ ] `select_worker` implements round-robin with probe-based skip
- [ ] All-workers-offline → job re-queued (status back to Queued, running_count decremented)
- [ ] `worker_host` set on `QueuedJob` before spawning SSH job task
- [ ] `#[allow(dead_code)]` removed from `ssh_timeout_secs`
- [ ] `run_ssh_job_task` calls deliver_manifest → run_remote_job → sync_state_back in sequence
- [ ] Unit tests for `select_worker` (3 cases) and re-queue path

## Verification

- `cargo test --workspace` — all pass, 0 failures
- `cargo test -p smelt-cli --lib -- dispatch::tests` — new unit tests pass
- `grep -c 'allow(dead_code)' crates/smelt-cli/src/serve/config.rs` returns 1 (only `retry_backoff_secs`)

## Observability Impact

- Signals added/changed: `tracing::info!` on SSH dispatch (job_id, worker_host); `tracing::warn!` on probe failure (host, error); `tracing::info!` on re-queue (job_id, reason: "all workers offline")
- How a future agent inspects this: grep logs for "dispatching job to worker", "probe failed for worker", "all workers offline — re-queueing"
- Failure state exposed: re-queued job retains `worker_host: None` (not assigned); probe failure logged with host + error

## Inputs

- `crates/smelt-cli/src/serve/dispatch.rs` — existing `dispatch_loop` and `run_job_task`
- `crates/smelt-cli/src/serve/ssh.rs` — `SshClient` trait, `deliver_manifest`, `run_remote_job`, `sync_state_back`, `MockSshClient`
- `crates/smelt-cli/src/serve/queue.rs` — `ServerState` struct
- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig`, `WorkerConfig`
- T01 output: `QueuedJob.worker_host` field

## Expected Output

- `crates/smelt-cli/src/serve/dispatch.rs` — `run_ssh_job_task`, `select_worker`, modified `dispatch_loop`
- `crates/smelt-cli/src/serve/queue.rs` — `round_robin_idx` on `ServerState`
- `crates/smelt-cli/src/serve/config.rs` — `#[allow(dead_code)]` removed from `ssh_timeout_secs`
- `crates/smelt-cli/src/serve/mod.rs` — updated re-exports if needed
- `crates/smelt-cli/src/commands/serve.rs` — wiring config into dispatch_loop call
