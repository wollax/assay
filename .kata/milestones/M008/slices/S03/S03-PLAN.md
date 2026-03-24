# S03: State sync back via scp

**Goal:** After `smelt run` completes on a remote worker, the dispatcher scps `.smelt/runs/<job_name>/` back to its local filesystem so `smelt status <job>` reads correct phase and exit code.
**Demo:** A mock-based unit test proves `sync_state_back()` calls `scp_from()` with the correct remote path and creates the local state directory; a gated integration test (`SMELT_SSH_TEST=1`) delivers a manifest to localhost, runs a command that produces state, syncs it back, and `JobMonitor::read()` returns the correct phase.

## Must-Haves

- `scp_from()` method on `SshClient` trait — symmetric with `scp_to()`, uses `-r` for recursive directory copy, reverses src/dest order
- `SubprocessSshClient::scp_from()` implementation using `build_scp_args()` with `-r` flag
- `MockSshClient` gains `scp_from_results` queue with `with_scp_from_result()` builder
- `sync_state_back<C: SshClient>()` free function — pulls `/tmp/.smelt/runs/<job_name>/` from worker to `local_dest_dir/.smelt/runs/<job_name>/`; creates local dirs via `create_dir_all`; returns `Err` on scp failure (caller decides how to handle)
- Unit tests via `MockSshClient` for success and failure paths
- Gated integration test (`SMELT_SSH_TEST=1`) proving full deliver → exec → sync-back → `JobMonitor::read()` round-trip
- `cargo test --workspace` all green

## Proof Level

- This slice proves: contract (unit tests via mock) + integration (gated localhost SSH test)
- Real runtime required: only for gated test (`SMELT_SSH_TEST=1`)
- Human/UAT required: no

## Verification

- `cargo test -p smelt-cli --lib -- ssh::tests::test_scp_from_args` — verifies `-r` flag and reversed src/dest order
- `cargo test -p smelt-cli --lib -- ssh::tests::test_sync_state_back_mock_success` — mock scp_from success path
- `cargo test -p smelt-cli --lib -- ssh::tests::test_sync_state_back_mock_failure` — mock scp_from failure returns Err
- `cargo test --workspace` — all tests pass, 0 failures
- Gated: `SMELT_SSH_TEST=1 cargo test -p smelt-cli -- --include-ignored test_state_sync_round_trip` — real localhost scp round-trip

## Observability / Diagnostics

- Runtime signals: `tracing::debug!` on `scp_from` entry (host, remote_src, local_dest); `tracing::warn!` on non-zero scp exit (host, exit_code, stderr)
- Inspection surfaces: `sync_state_back` returns `Result<()>` — caller in S04 dispatch can log warning and set job status
- Failure visibility: scp stderr captured in error message; job_name and worker host in log context
- Redaction constraints: key_env resolved path only in DEBUG logs (D112)

## Integration Closure

- Upstream surfaces consumed: `SshClient` trait + `SubprocessSshClient` + `build_scp_args()` + `MockSshClient` (S01/S02); `WorkerConfig` (S01); `JobMonitor::read()` (smelt-core)
- New wiring introduced in this slice: `scp_from()` trait method; `sync_state_back()` free function
- What remains before the milestone is truly usable end-to-end: S04 must call `sync_state_back()` in the dispatch loop after `run_remote_job()` completes; S04 must parse `job_name` from the manifest to pass as parameter; S04 wires dispatch routing, round-robin, and `worker_host` field

## Tasks

- [x] **T01: Add scp_from() to SshClient trait with recursive copy and MockSshClient extension** `est:20m`
  - Why: S03's `sync_state_back()` needs a reverse-direction scp method; the mock needs a separate result queue for test isolation
  - Files: `crates/smelt-cli/src/serve/ssh.rs`
  - Do: Add `scp_from(&self, worker, timeout_secs, remote_src, local_dest) -> Result<()>` to trait; implement in `SubprocessSshClient` using `build_scp_args()` with `-r` prepended to extra_args and remote-first arg order (`user@host:/path` then local path); add `scp_from_results` queue to `MockSshClient` with `with_scp_from_result()` builder; add unit tests for scp_from args (verify `-r` present, src/dest order correct) and mock-based scp_from success/failure
  - Verify: `cargo test -p smelt-cli --lib -- ssh::tests::test_scp_from` passes; `cargo test --workspace` all green
  - Done when: `scp_from()` on trait + SubprocessSshClient impl + MockSshClient queue + 3 new unit tests passing

- [x] **T02: Implement sync_state_back() free function with unit tests and gated integration test** `est:25m`
  - Why: This is the boundary contract S04 consumes — a single function that pulls remote job state to the dispatcher's local filesystem
  - Files: `crates/smelt-cli/src/serve/ssh.rs`, `crates/smelt-cli/src/serve/tests.rs`
  - Do: Add `sync_state_back<C: SshClient>(client, worker, timeout_secs, job_name, local_dest_dir) -> Result<()>` — computes remote path as `/tmp/.smelt/runs/<job_name>/`, local path as `local_dest_dir/.smelt/runs/<job_name>/`, calls `create_dir_all` on local path parent, calls `client.scp_from()` with `-r`; add mock-based unit tests (success + failure); add gated integration test in `tests.rs` that delivers a manifest, creates remote state via `ssh mkdir + echo`, syncs back, verifies with `JobMonitor::read()` or file existence
  - Verify: `cargo test -p smelt-cli --lib -- ssh::tests::test_sync_state_back` passes; `cargo test --workspace` all green; gated test present and `#[ignore]`d
  - Done when: `sync_state_back()` exported from ssh module; 2+ mock unit tests passing; 1 gated integration test present; `cargo test --workspace` all green

## Files Likely Touched

- `crates/smelt-cli/src/serve/ssh.rs` — scp_from trait method, SubprocessSshClient impl, MockSshClient extension, sync_state_back free function, unit tests
- `crates/smelt-cli/src/serve/tests.rs` — gated integration test for full round-trip
