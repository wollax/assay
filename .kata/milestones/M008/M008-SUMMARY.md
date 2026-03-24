---
id: M008
provides:
  - WorkerConfig struct with host/user/key_env/port fields, deny_unknown_fields, serde defaults
  - ServerConfig::workers Vec<WorkerConfig> and ssh_timeout_secs fields
  - SshClient trait with exec(), probe(), scp_to(), scp_from() — generic (RPITIT), not object-safe
  - SubprocessSshClient implementation using ssh/scp subprocesses via tokio::process::Command
  - build_ssh_args() and build_scp_args() helpers for SSH/SCP flag composition
  - deliver_manifest() — scps manifest to /tmp/smelt-<job_id>.toml on worker
  - run_remote_job() — SSHes `smelt run <path>`, returns exit code
  - sync_state_back() — scps .smelt/runs/<job_name>/ from worker to dispatcher
  - MockSshClient with configurable exec/scp/probe/scp_from result queues
  - dispatch_loop SSH routing — local when no workers, SSH when workers configured
  - select_worker() — round-robin probe loop returning first live worker or None
  - run_ssh_job_task() — full SSH dispatch lifecycle (deliver → exec → sync → complete)
  - round_robin_idx on ServerState for worker selection
  - worker_host: Option<String> on QueuedJob, JobStateResponse, and TUI Worker column
  - examples/server.toml [[workers]] documented configuration block
key_decisions:
  - "D111: subprocess ssh/scp not openssh/ssh2 crate — consistent with D002 (git CLI pattern)"
  - "D112: key_env stores env var name only, never key value — follows D014 credential injection"
  - "D121: SshClient uses generics not dyn — RPITIT async fn not object-safe"
  - "D122: dispatch_loop generic over SshClient for testability"
  - "D123: round_robin_idx is volatile (not serialized) — resets to 0 on restart"
  - "D124: All-workers-offline re-queues job without incrementing attempt count"
patterns_established:
  - "SshClient trait + SubprocessSshClient unit struct — mirrors GitOps + GitCli pattern (D060)"
  - "build_ssh_args()/build_scp_args() — composable arg builders for unit testing without real SSH"
  - "MockSshClient with per-method VecDeque result queues for independent test configuration"
  - "deliver_manifest + run_remote_job + sync_state_back as composable free functions generic over SshClient"
  - "SSH dispatch pattern: select_worker probe → set worker_host → spawn run_ssh_job_task"
  - "Gated integration tests: SMELT_SSH_TEST=1 env var guard in #[tokio::test] #[ignore]"
observability_surfaces:
  - "GET /api/v1/jobs returns worker_host per job (null for local, string for remote)"
  - "TUI Worker column shows host or '-'"
  - ".smelt-queue-state.toml persists worker_host via QueuedJob serde"
  - "tracing::debug/warn on SSH exec, scp_to, scp_from, sync_state_back entry/failure"
  - "tracing::info on SSH dispatch entry (job_id, worker_host) and re-queue (all workers offline)"
  - "tracing::warn on probe failure with host context and on exit code 127 (smelt not on PATH hint)"
requirement_outcomes:
  - id: R027
    from_status: active
    to_status: validated
    proof: "S01 proves WorkerConfig parsing + SshClient + probe timeout (8 tests). S02 proves manifest delivery + remote exec (7 tests, MockSshClient). S03 proves state sync back (5 tests). S04 proves dispatch routing, round-robin, failover, worker_host in API/TUI (4 dispatch tests + 2 API tests + 1 TUI test). 286 workspace tests green (81 smelt-cli + 155 smelt-core + others), 0 failures. Live multi-host proof deferred to S04-UAT.md."
duration: 2h
verification_result: passed
completed_at: 2026-03-24T13:30:00Z
---

# M008: SSH Worker Pools

**Full SSH dispatch pipeline: `[[workers]]` config → subprocess ssh/scp → manifest delivery → remote `smelt run` → state sync back → round-robin routing with probe-based failover → worker_host visible in API and TUI — 286 workspace tests green**

## What Happened

M008 extended `smelt serve` to dispatch jobs to remote machines over SSH, transforming the dispatcher from a single-host daemon into a multi-machine job distribution system.

**S01 (WorkerConfig + SSH connection proof)** laid the foundation: `WorkerConfig` struct with `host`, `user`, `key_env`, `port` fields in `server.toml`'s `[[workers]]` table; `SshClient` trait with async `exec()` and `probe()` methods; `SubprocessSshClient` implementation using `tokio::process::Command` with `which::which("ssh")` for binary discovery. The SSH subprocess approach (D111) was chosen for consistency with the existing git CLI pattern (D002), respecting `~/.ssh/config` and ssh-agent automatically. `build_ssh_args()` assembles `BatchMode=yes`, `StrictHostKeyChecking=accept-new`, and `ConnectTimeout` flags. Probe fast-fail is delegated entirely to SSH's own `ConnectTimeout`.

**S02 (Manifest delivery + remote exec)** extended `SshClient` with `scp_to()` for file delivery, built `build_scp_args()` (uppercase `-P` for port — a critical scp vs ssh difference), and implemented two composable free functions: `deliver_manifest()` (scps manifest to `/tmp/smelt-<job_id>.toml`) and `run_remote_job()` (SSHes `smelt run <path>`, returns exit code). `MockSshClient` was created with per-method `VecDeque` result queues for isolated test configuration.

**S03 (State sync back)** added the symmetric `scp_from()` trait method for recursive remote-to-local directory copy and `sync_state_back()` — pulls `.smelt/runs/<job_name>/` from worker to dispatcher's local filesystem so `smelt status <job>` works normally on the dispatcher.

**S04 (Dispatch routing + round-robin + TUI/API)** wired all primitives into the dispatch loop. `dispatch_loop` gained generic `<C: SshClient>` parameter for testability (D122). `select_worker()` probes workers round-robin from `round_robin_idx` on `ServerState`; first responsive worker gets the job via `run_ssh_job_task()` (deliver → exec → sync → complete). When all workers are offline, the job is re-queued without incrementing attempt count (D124). `worker_host: Option<String>` flows through `QueuedJob` → `JobStateResponse` (API) → TUI Worker column. The `SshClient` trait was changed from `async fn` to `fn -> impl Future + Send` for `tokio::spawn` compatibility.

## Cross-Slice Verification

| Success Criterion | Status | Evidence |
|---|---|---|
| `[[workers]]` entries in `server.toml` cause SSH dispatch | ✅ PASS | `WorkerConfig` struct + `ServerConfig::workers` parses; `dispatch_loop` routes to SSH when workers non-empty; 6 config tests + `test_round_robin_two_workers` integration test |
| Job submitted via POST/dir-watch executes on remote worker | ✅ PASS | `run_ssh_job_task` orchestrates deliver_manifest → run_remote_job → sync_state_back → complete; MockSshClient integration tests prove full lifecycle |
| `smelt status <job>` shows correct phase/exit/elapsed for remote jobs | ✅ PASS | `sync_state_back()` copies `.smelt/runs/<job>/` to dispatcher; proven by `test_state_sync_round_trip` gated test and `test_sync_state_back_mock_success` |
| `worker_host` visible in `GET /api/v1/jobs` and TUI | ✅ PASS | `test_worker_host_in_api_response`, `test_worker_host_none_in_api_response`, `test_tui_render_worker_host` |
| Unreachable worker → re-queue to another worker | ✅ PASS | `test_select_worker_one_offline_skip`, `test_select_worker_all_offline`, `test_requeue_all_workers_offline`, `test_failover_one_offline` |
| `smelt run` direct invocation unchanged — zero regressions | ✅ PASS | `cargo test --workspace` — 286 passed, 0 failed, 9 ignored |
| `examples/server.toml` documents `[[workers]]` | ✅ PASS | Commented `[[workers]]` block present in examples/server.toml |

**Definition of Done verification:**
- All 4 slices `[x]` in ROADMAP.md ✅
- All 4 slice summaries exist ✅
- `cargo test --workspace` — 286 passed, 0 failed ✅
- Cross-slice integration: S04 consumes all S01–S03 primitives (WorkerConfig, SshClient, deliver_manifest, run_remote_job, sync_state_back) in dispatch_loop ✅
- `worker_host` plumbed through types → API → TUI ✅
- Only 1 `allow(dead_code)` remaining (retry_backoff_secs — pre-existing, not M008) ✅

## Requirement Changes

- R027: active → validated — All 4 slices complete. SSH subprocess approach proven (D111). Config parsing (6 tests), SSH arg building (4 tests), mock-based delivery/exec/sync (7 tests), dispatch routing with round-robin and failover (4 tests), API/TUI worker_host visibility (3 tests). 286 workspace tests green. Live multi-host proof deferred to S04-UAT.md.

## Forward Intelligence

### What the next milestone should know
- The SSH dispatch pipeline is: `select_worker` (probe round-robin) → `set worker_host` → `spawn run_ssh_job_task` (deliver_manifest → run_remote_job → sync_state_back → complete)
- `SshClient` trait uses `fn -> impl Future + Send` (not `async fn`) — required for `tokio::spawn` compatibility
- `MockSshClient` is mature: per-method result queues, `with_exec_result`/`with_scp_result`/`with_probe_result`/`with_scp_from_result` builders — use it for any SSH-related test extensions
- `dispatch_loop` is generic over `<C: SshClient + Send + Sync + 'static>` — test with MockSshClient, production with SubprocessSshClient

### What's fragile
- `SubprocessSshClient` depends on `ssh`/`scp` binaries being on PATH — no startup check, fails at first dispatch
- `build_scp_args()` uses uppercase `-P` for port while `build_ssh_args()` uses lowercase `-p` — any refactoring must preserve this difference
- Remote state directory path `/tmp/.smelt/runs/<job_name>/` must match what `smelt run` produces on the worker — if state directory structure changes, sync breaks silently
- `which::which("ssh")` / `which::which("scp")` will fail in minimal container environments without OpenSSH client installed

### Authoritative diagnostics
- `cargo test -p smelt-cli --lib -- ssh::tests` — 12 SSH tests in <1s, fastest signal for SSH module regressions
- `cargo test -p smelt-cli --lib -- dispatch::tests` — 4 dispatch routing tests for round-robin/failover logic
- `cargo test -p smelt-cli --lib -- serve::tests` — full serve integration test suite
- Runtime logs: grep for `dispatching job to worker`, `probe failed`, `all workers offline` in `.smelt/serve.log`

### What assumptions changed
- `SshClient` trait needed `fn -> impl Future + Send` instead of `async fn` — `tokio::spawn` requires `Send` futures and RPITIT `async fn in trait` doesn't guarantee `Send`
- `probe()` was originally specified as a separate TCP-level probe but implemented as `exec("echo smelt-probe")` — simpler, SSH's own ConnectTimeout provides the same ≤3s guarantee

## Files Created/Modified

- `crates/smelt-cli/src/serve/ssh.rs` — New: SshClient trait, SubprocessSshClient, MockSshClient, build_ssh_args/build_scp_args, deliver_manifest, run_remote_job, sync_state_back, 14 tests
- `crates/smelt-cli/src/serve/config.rs` — WorkerConfig struct, ServerConfig::workers + ssh_timeout_secs, extended validate()
- `crates/smelt-cli/src/serve/dispatch.rs` — SSH dispatch path: select_worker, run_ssh_job_task, generic dispatch_loop
- `crates/smelt-cli/src/serve/types.rs` — worker_host: Option<String> on QueuedJob
- `crates/smelt-cli/src/serve/http_api.rs` — worker_host in JobStateResponse + 2 tests
- `crates/smelt-cli/src/serve/tui.rs` — Worker column + test
- `crates/smelt-cli/src/serve/queue.rs` — round_robin_idx on ServerState
- `crates/smelt-cli/src/serve/mod.rs` — pub mod ssh + re-exports
- `crates/smelt-cli/src/commands/serve.rs` — Wired workers, SubprocessSshClient, ssh_timeout_secs into dispatch_loop
- `crates/smelt-cli/src/serve/tests.rs` — 4 integration tests (round-robin, failover, all-offline, worker_host persistence)
- `examples/server.toml` — Commented [[workers]] example block
