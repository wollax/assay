# M008: SSH Worker Pools

**Vision:** `smelt serve` dispatches jobs to remote machines via SSH — the dispatcher runs locally, workers run `smelt run` on their own Docker/K8s stack, manifests are delivered via scp, and job state syncs back so `smelt status` works normally on the dispatcher. Workers are statically configured in `server.toml`; jobs are round-robined across available workers.

## Success Criteria

- Adding `[[workers]]` entries to `server.toml` causes `smelt serve` to dispatch jobs to those remote hosts instead of running them locally
- A job submitted via `POST /api/v1/jobs` or directory watch is executed by `smelt run` on a remote worker and its state syncs back to the dispatcher
- `smelt status <job>` on the dispatcher shows correct phase, exit code, and elapsed time — even though the job ran remotely
- Worker host is visible in `GET /api/v1/jobs` and the TUI (`worker_host` field)
- If a configured worker is unreachable at dispatch time, the job is re-queued and dispatched to another available worker on the next tick
- `smelt run manifest.toml` direct invocation unchanged — zero regressions in `cargo test --workspace`
- `examples/server.toml` documents `[[workers]]` configuration

## Key Risks / Unknowns

- **SSH execution approach** — subprocess `ssh`/`scp` vs `openssh` crate vs `ssh2` crate. Subprocess is lowest-friction and consistent with the `git` CLI pattern (D002) but adds process spawning overhead. Need to evaluate in S01 before committing.
- **Offline worker at dispatch time** — the dispatcher must detect a failed SSH connection quickly (2-5s timeout) and re-queue the job rather than hanging. This requires a fast-fail SSH probe pattern. Must be proven before S02 builds on it.
- **State sync reliability** — if scp of `.smelt/runs/<job>/` fails after `smelt run` completes, the dispatcher has no state for that job. This is a known limitation (log warn, don't retry the job). Acceptable for M008 but must be documented.

## Proof Strategy

- **SSH execution approach** → retire in S01 by proving: connect to localhost SSH, copy a file via scp, exec a command, read stdout/exit code — all via chosen approach
- **Offline worker detection** → retire in S01 by proving: connection to a port that refuses → error returned within 3s → dispatch loop continues unblocked
- **End-to-end state sync** → retire in S04 by running a real job through the full dispatch → remote exec → scp-back → `smelt status` pipeline

## Verification Classes

- Contract verification: `[[workers]]` config parses and validates; `WorkerConfig` struct + `ServerConfig::workers` field; SSH connection to localhost proves the library/subprocess works
- Integration verification: manifest delivered to worker, `smelt run` executes on remote, `.smelt/runs/<job>/` synced back, `GET /api/v1/jobs` shows correct state
- Operational verification: worker offline → re-queue within one dispatch tick; two workers → round-robin assignment; one worker down → all jobs route to surviving worker
- UAT / human verification: manual run with a real remote host (SSH into another machine), verify TUI shows `worker_host`, verify `smelt status` works on dispatcher

## Milestone Definition of Done

This milestone is complete only when all are true:

- `[[workers]]` table parses in `server.toml` with host/user/key_env/port fields
- SSH connection, manifest delivery (scp), remote smelt run execution, and state sync-back are all implemented and tested
- Dispatch routing: local execution when no workers configured; SSH dispatch when workers present
- Round-robin worker selection with offline-worker skip and re-queue
- `worker_host` visible in `GET /api/v1/jobs` response and TUI
- Integration tests with localhost SSH (or mock) pass
- `cargo test --workspace` all green

## Requirement Coverage

- Covers: R027 (SSH worker pools / remote dispatch)
- Partially covers: none
- Leaves for later: R026 (Linear/GitHub Issues backlog integration), R022 (budget/cost tracking)
- Orphan risks: none

## Slices

- [x] **S01: WorkerConfig + SSH connection proof** `risk:high` `depends:[]`
  > After this: `[[workers]]` parses from `server.toml`; SSH connection to localhost (or a configurable test host) established; a test command executes and stdout is captured; offline-worker returns error within 3s — all proven by unit/integration tests.

- [x] **S02: Manifest delivery + remote smelt run execution** `risk:high` `depends:[S01]`
  > After this: given a manifest TOML path, the dispatcher scps it to `/tmp/smelt-<job_id>.toml` on the worker and SSHes `smelt run /tmp/smelt-<job_id>.toml`; exit code is captured and mapped to job success/failure; integration test with a real localhost SSH session proves the full delivery+exec path.

- [ ] **S03: State sync back via scp** `risk:medium` `depends:[S02]`
  > After this: after `smelt run` completes on the worker, dispatcher scps `.smelt/runs/<job>/` back to its own filesystem; `smelt status <job>` on the dispatcher reads the synced state and shows correct phase; scp failure logs a warning but does not re-run the job.

- [ ] **S04: Dispatch routing + round-robin + TUI/API worker field** `risk:medium` `depends:[S01,S02,S03]`
  > After this: `dispatch_loop` routes to SSH workers when `config.workers` is non-empty, falls back to local when empty; round-robin index tracked in `ServerState`; offline worker re-queues job; `worker_host` field in `JobStateResponse` and TUI; end-to-end integration test with 2 mock workers confirms round-robin and failover.

## Boundary Map

### S01 → S02, S03, S04

Produces:
- `WorkerConfig { host: String, user: String, key_env: String, port: u16 }` with `#[derive(Deserialize)]`
- `ServerConfig::workers: Vec<WorkerConfig>` — empty vec when absent (no workers = local dispatch)
- `SshClient` (or equivalent) — connects to a worker, executes a command, returns (stdout, exit_code); returns error within 3s if host unreachable
- `examples/server.toml` updated with commented `[[workers]]` block

Consumes:
- nothing (first slice)

### S02 → S03, S04

Produces:
- `deliver_manifest(worker, job_id, manifest_path) -> Result<RemoteManifestPath>` — scps manifest to `/tmp/smelt-<job_id>.toml` on worker
- `run_remote_job(worker, remote_manifest_path) -> Result<i32>` — SSHes `smelt run <path>`, captures exit code
- Integration test: localhost SSH session, manifest delivered and `echo hello` (or real `smelt run --dry-run`) executed

Consumes from S01:
- `SshClient`, `WorkerConfig`

### S03 → S04

Produces:
- `sync_state_back(worker, job_id, local_state_dir) -> Result<()>` — scps `.smelt/runs/<job_id>/` from worker to dispatcher's local path; warns on failure
- Integration test: run → sync → `smelt status <job>` reads synced state

Consumes from S01:
- `SshClient`, `WorkerConfig`

Consumes from S02:
- `run_remote_job` (state sync triggered after it returns)

### S04 (final wiring — no new public surfaces)

Produces:
- Dispatch routing layer in `dispatch_loop`: `if config.workers.is_empty() { local } else { ssh_dispatch }`
- Round-robin worker index in `ServerState`; offline-worker skip with re-queue
- `worker_host: Option<String>` in `QueuedJob` and `JobStateResponse`
- TUI updated to show worker_host column (or append to status cell)
- End-to-end integration test: 2 mock workers, 4 jobs, assert round-robin + failover

Consumes from S01:
- `WorkerConfig`, `SshClient`, `ServerConfig::workers`

Consumes from S02:
- `deliver_manifest`, `run_remote_job`

Consumes from S03:
- `sync_state_back`
