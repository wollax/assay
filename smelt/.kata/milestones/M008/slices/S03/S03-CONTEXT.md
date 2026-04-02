---
id: S03
milestone: M008
status: ready
---

# S03: State sync back via scp — Context

## Goal

Introduce a `StateBackend` trait with a `FileScp` implementation that pulls `.smelt/runs/<job_name>/` from the worker to `queue_dir/.smelt/runs/<job_name>/` on the dispatcher after `smelt run` completes, so `smelt status <job>` works on the dispatcher.

## Why this Slice

S02 proves the remote exec path; S04 needs state visible on the dispatcher to close the end-to-end loop. Rather than hard-coding scp-back, S03 introduces the `StateBackend` abstraction now — S04 wires against the trait, and future milestones can add Linear/GitHub/GitLab/Azure DevOps backends without changing the dispatch layer.

## Scope

### In Scope

- `StateBackend` trait with two methods: `push(job_id, state_dir)` and `pull(job_id, dest_dir)` — minimal surface covering both directions of state movement
- `FileScp` impl: `pull` uses `scp -r` from `<user>@<host>:/tmp/.smelt/runs/<job_id>/` to `queue_dir/.smelt/runs/<job_name>/` on the dispatcher; `push` is a stub or symmetric impl (remote-to-local direction used in S03, local-to-remote available for future use)
- Sync triggered after `run_remote_job()` returns — fire-and-forget: on scp failure, WARN log + job marked **Failed**; no retry of the sync itself
- State landing path: `queue_dir/.smelt/runs/<job_name>/` — anchored to the daemon's `queue_dir`, consistent with existing dispatcher state layout
- Unit tests via `MockSshClient` (no real SSH required): successful pull, failed pull → WARN + Failed transition
- Gated integration test (`SMELT_SSH_TEST=1`): full flow — deliver manifest → run remote job → pull state back → `JobMonitor::read()` from synced dir returns correct phase

### Out of Scope

- Linear, GitHub, GitLab, Azure DevOps, or any non-file state backend (future milestones)
- `push` direction being exercised at runtime in M008 (only `pull` is called by S04 dispatch)
- Retry or re-attempt of a failed state sync
- Per-worker `remote_state_dir` configuration (not needed for the scp path; future backends will have their own config)
- Changing what `smelt status` or the HTTP API display for sync-failed jobs beyond the existing Failed status (that's S04's concern)

## Constraints

- `StateBackend` must be generic over `SshClient` (D121 pattern) to remain unit-testable without real SSH
- State sync failure marks the job **Failed** — honest failure is preferred over optimistic Complete; operator checks `serve.log` for the scp error with job_id and worker host
- The synced state dir must be at `queue_dir/.smelt/runs/<job_name>/` so `JobMonitor::read()` and `smelt status <job>` find it without additional config
- On the worker, `smelt run /tmp/smelt-<job_id>.toml` writes state to `/tmp/.smelt/runs/<job_name>/state.toml` — that is the path to pull from (relative to the manifest parent `/tmp/`)
- D111 (subprocess scp), D112 (key_env), D121 (generics) all apply; `scp_to()` from S02 is the model for the reverse direction

## Integration Points

### Consumes

- `SshClient::scp_to()` (S02) — used as the model; a symmetric `scp_from()` method may be added to the trait, or the reverse direction can be assembled from the same `build_scp_args()` helper with swapped src/dst
- `MockSshClient` (S02) — builder pattern for unit tests
- `WorkerConfig` (S01) — host, user, key_env, port for scp invocation
- `JobMonitor::read(state_dir)` (smelt-core) — used in the integration test to verify synced state is parseable

### Produces

- `StateBackend` trait — `push(job_id, local_state_dir) -> Result<()>` and `pull(job_id, local_dest_dir) -> Result<()>`; generic over `C: SshClient`
- `FileScp` struct implementing `StateBackend` — holds a worker ref, ssh client, timeout_secs
- `sync_state_back<C: SshClient>(worker, job_id, local_dest_dir) -> Result<()>` free function (or delegating to `FileScp::pull`) consumed by S04 dispatch
- Unit tests: `test_sync_state_back_mock_success`, `test_sync_state_back_mock_failure`
- Gated integration test: `test_state_sync_full_round_trip` (`SMELT_SSH_TEST=1`)

## Open Questions

- **`scp_from()` on trait vs assembling from `build_scp_args()`** — adding `scp_from()` to `SshClient` is the clean path (mirrors `scp_to()`); alternatively the `FileScp` impl can assemble the reverse scp command using `build_scp_args()` directly without extending the trait. Leaning toward adding `scp_from()` for symmetry and testability, but confirm at planning time.
- **`StateBackend` generic parameter** — `StateBackend<C: SshClient>` as a generic struct, or `StateBackend` as a trait with associated type? The generic struct approach mirrors `deliver_manifest<C>` and `run_remote_job<C>`; associated type is more flexible for non-SSH backends. Decide at planning — leaning toward generic struct for S03 simplicity.
- **Trait home** — `StateBackend` could live in `ssh.rs` (alongside the other SSH abstractions) or in a new `state.rs` module. A new `state.rs` is cleaner if future backends are non-SSH. Decide at planning.
