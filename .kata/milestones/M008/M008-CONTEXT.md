# M008: SSH Worker Pools — Context

**Gathered:** 2026-03-23
**Status:** Ready for planning

## Project Description

Smelt is the infrastructure layer in the smelt/assay/cupel agentic development toolkit. M001–M007 deliver single and parallel job execution locally and persistent queue. M008 extends `smelt serve` to dispatch jobs to remote machines over SSH — the dispatcher runs locally, remote workers run `smelt run` on their own Docker/K8s stack, and state syncs back so `smelt status` works normally on the dispatcher.

## Why This Milestone

Local parallel dispatch (M006) is limited by single-machine resources. Users running large agentic workloads — many concurrent Assay sessions — need to spread work across multiple machines. SSH is the lowest-friction remote execution primitive: no new infrastructure, no cloud provider dependency, just `~/.ssh/id_rsa` and a reachable host.

M008 is the natural successor to M007: a persistent queue ensures jobs survive the dispatcher restart, and SSH workers ensure jobs can be run on any reachable host. Together they make `smelt serve` a real headless agentic infrastructure daemon.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Add `[[workers]]` entries to `server.toml` with `host`, `user`, and `key_env` (env var holding the SSH private key path)
- Start `smelt serve --config server.toml` and watch jobs dispatched to remote workers in round-robin order instead of running locally
- See remote job state in the TUI and `GET /api/v1/jobs` just like local jobs
- Run `smelt status <job>` on the dispatcher and see the job's phase, exit code, and PR URL — even though it ran on a remote machine

### Entry point / environment

- Entry point: `smelt serve --config server.toml` (existing subcommand, extended)
- Environment: dispatcher on any host; workers must have `smelt` installed and Docker/K8s available
- Live dependencies: SSH access from dispatcher to workers; `smelt` binary on workers; Docker or K8s on workers

## Completion Class

- Contract complete means: `[[workers]]` config parses and validates; SSH connection to a worker can be established and a test command run; manifest delivered and `smelt run` invoked
- Integration complete means: dispatcher submits a real job to a remote worker, `smelt run` executes on the remote, state syncs back, `smelt status <job>` shows correct phase on the dispatcher
- Operational complete means: two workers in config, jobs dispatched round-robin, one worker goes offline mid-run (connection refused) → job re-queued to other worker

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- Integration test: two mock workers (or real localhost SSH), dispatcher dispatches 2 jobs, both complete, state syncs back, `GET /api/v1/jobs` shows both Complete
- One worker fails (connection refused) during dispatch → job re-queued, dispatched to other worker on next tick
- `smelt status <job>` on the dispatcher reads `.smelt/runs/<job>/state.toml` synced from the remote
- `smelt run manifest.toml` direct invocation unchanged — zero regressions

## Risks and Unknowns

- **SSH library choice** — `openssh` crate (async, tokio-native) vs `ssh2` crate (libssh2 bindings, sync) vs shell out to system `ssh`/`scp`. The right choice depends on whether we need async multiplexing or whether sequential manifest-delivery + exec + scp is acceptable. Need to evaluate before S01.
- **Manifest delivery** — the manifest TOML must be copied to the remote before `smelt run` can execute. Options: (a) `scp` to a temp path on the remote, (b) base64-encode and `echo | base64 -d` via SSH exec (mirrors D028), (c) write to stdin of the remote process. Option (a) is cleanest.
- **State sync back** — `.smelt/runs/<job>/` must be copied back from the remote to the dispatcher after `smelt run` completes. Options: (a) `scp -r` after job completion, (b) mount a shared NFS path (out of scope), (c) have the remote push to a known dispatcher HTTP endpoint (overengineered). Option (a) is correct.
- **Worker availability detection** — if a worker is unreachable at dispatch time, the dispatcher should log a warning, re-queue the job, and try again on the next tick. This requires a fast connection timeout (2-5s) to avoid blocking the dispatch loop.
- **Worker selection strategy** — round-robin is specified. Need to track the last-used worker index in `ServerState`. Must handle workers going offline gracefully (skip, not panic).

## Existing Codebase / Prior Art

- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig`; needs `[[workers]]` table added; `deny_unknown_fields` means adding fields requires explicit struct change
- `crates/smelt-cli/src/serve/dispatch.rs` — `dispatch_loop`/`run_job_task`; `run_job_task` currently calls `run_with_cancellation()` directly; needs a dispatch routing layer
- `crates/smelt-cli/src/serve/queue.rs` — `ServerState`; needs worker assignment tracking (round-robin index)
- `crates/smelt-cli/src/commands/run.rs` — `run_with_cancellation()` is the local execution entry point; SSH dispatch bypasses this and calls `smelt run` on the remote
- `crates/smelt-core/src/git/cli.rs` — `GitCli` uses subprocess `git` calls; SSH execution is similar in pattern
- `examples/server.toml` — canonical config; will gain a `[[workers]]` section

> See `.kata/DECISIONS.md` for relevant decisions: D098 (in-process vs subprocess), D048 (docker cp for large binary injection — similar pattern for manifest delivery), D023 (teardown guarantee).

## Relevant Requirements

- R027 — SSH worker pools / remote dispatch: this milestone fully implements it

## Scope

### In Scope

- `[[workers]]` in `server.toml`: `host: String`, `user: String`, `key_env: String` (env var holding path to SSH private key), optional `port: u16` (default 22)
- SSH connection establishment and health check at serve startup (warn on unreachable workers, don't fail)
- Manifest delivery to remote: scp manifest TOML to temp path on worker
- Remote job execution: SSH exec `smelt run <manifest_path>` on the worker
- State sync back: scp `.smelt/runs/<job>/` from worker to dispatcher after completion
- Round-robin worker selection with offline-worker skip (re-queue if no worker available)
- Worker assignment visible in TUI and `GET /api/v1/jobs` response (`worker_host` field)
- `examples/server.toml` updated with documented `[[workers]]` entries
- Integration tests with localhost SSH (or mock SSH interface)

### Out of Scope / Non-Goals

- Dynamic worker discovery (workers must be statically listed in config)
- Worker autoscaling or cloud provisioning
- Worker authentication beyond SSH key (no password auth, no OIDC)
- Shared filesystem / NFS (each job delivers its own manifest and syncs its own state)
- Installing `smelt` on remote workers (user's responsibility)
- Windows remote workers
- Multiplexed SSH sessions (`ControlMaster`) — one connection per job is acceptable

## Technical Constraints

- D098: dispatcher still runs `dispatch_loop` in-process; the SSH path replaces the local `run_job_task` tokio future, not the dispatch architecture
- SSH private key is read from an env var (key_env field) — never stored in config file
- Connection timeout must be short (2-5s) to avoid blocking the dispatch tick for offline workers
- `smelt run` must be on the PATH of the remote user's non-interactive SSH shell (or specified as an absolute path)
- State sync is fire-and-forget after job completion — if scp fails, log a warning but don't re-run the job

## Integration Points

- SSH daemon on worker hosts — port 22, public-key auth
- `smelt` binary on workers — must be installed and on PATH
- `smelt-cli/src/serve/dispatch.rs` — routing layer added before `run_job_task`
- `smelt-cli/src/serve/config.rs` — `[[workers]]` table
- `smelt-core/src/monitor.rs` — `JobMonitor` state path synced back from worker

## Open Questions

- **SSH library**: `openssh` (tokio-native, async) vs `ssh2` (libssh2 bindings) vs subprocess `ssh`/`scp`? Leaning toward subprocess `ssh`/`scp` to avoid a new native library dependency and keep the pattern consistent with `git` CLI (D002). To be confirmed in S01 research.
- **Temp path on worker**: use `/tmp/smelt-<job_id>.toml` for manifest delivery — simple, no cleanup needed (deleted by OS).
- **Absolute path for smelt on worker**: should `server.toml` allow `smelt_bin: String` per worker (default: `smelt`)? Deferred — user can alias or symlink; not needed for M008.
