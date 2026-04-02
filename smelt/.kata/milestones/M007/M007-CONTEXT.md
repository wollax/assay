# M007: Persistent Queue — Context

**Gathered:** 2026-03-23
**Status:** Ready for planning

## Project Description

Smelt is the infrastructure layer in the smelt/assay/cupel agentic development toolkit. M001–M006 delivered single-job execution across three runtimes, GitHub PR lifecycle, per-job state tracking, and a parallel dispatch daemon (`smelt serve`). M007 adds crash-safe persistence to the `smelt serve` job queue: queue state survives restarts, and jobs that were queued or retrying when the daemon died are automatically re-dispatched on the next startup.

## Why This Milestone

`smelt serve`'s queue is entirely in-memory. A crash, SIGKILL, or deliberate restart drops every queued and in-flight job. For unattended autonomous workflows (the target use case), this means a host reboot or daemon OOM kills queued work silently. M007 closes this gap with minimal complexity: serialize queue state to a file on every state transition and load it on startup.

M007 does not require any new service dependencies — no Redis, no Postgres, no separate process. The `queue_dir` is already a filesystem-backed location; a `.smelt-queue-state.toml` in that directory is sufficient. Volume-backing the `queue_dir` (Docker named volume, K8s PV) is the user's responsibility; Smelt just guarantees the file is there and current.

## User-Visible Outcome

### When this milestone is complete, the user can:

- Run `smelt serve --config server.toml`, enqueue 5 jobs, kill the daemon with `kill -9`, restart it, and watch all 5 jobs dispatch and complete — no re-submission required
- Inspect `queue_dir/.smelt-queue-state.toml` and see a human-readable snapshot of all job states
- Configure `queue_dir` to point at a Docker named volume in a Compose stack and get persistent queue semantics across container restarts without any other changes

### Entry point / environment

- Entry point: `smelt serve --config server.toml` (existing subcommand, extended behavior)
- Environment: local dev or headless server; queue_dir on any persistent filesystem (plain dir, Docker volume, K8s PV)
- Live dependencies: none new — queue_dir filesystem only

## Completion Class

- Contract complete means: `QueuedJob`, `JobStatus`, `JobId`, `JobSource` all derive `Serialize + Deserialize`; `Instant` replaced with `SystemTime` (wall-clock) for serialization; `queue_state.toml` round-trips losslessly; unit tests prove write-on-transition and load-on-startup
- Integration complete means: `smelt serve` writes `queue_state.toml` on each enqueue/complete/cancel, loads it on startup, and re-dispatches surviving jobs
- Operational complete means: kill-and-restart with queued jobs in flight → jobs re-dispatch correctly; no duplicate dispatches; `smelt status <job>` still works for re-dispatched jobs

## Final Integrated Acceptance

To call this milestone complete, we must prove:

- Integration test: start `smelt serve` (no-op dispatch or mock), enqueue 3 jobs, stop the process, restart from the same config, assert all 3 jobs were re-queued and dispatch is attempted
- `Dispatching`/`Running` jobs at crash time are re-queued (not lost, not marked Failed) on restart
- `smelt status <job>` still reads `.smelt/runs/<job>/state.toml` for a re-dispatched job — no regression
- `cargo test --workspace` all green after changes

## Risks and Unknowns

- **`Instant` is not serializable** — `QueuedJob` uses `std::time::Instant` for `queued_at` and `started_at`. Must replace with `SystemTime` (or u64 Unix epoch) before serialization is possible. This touches `types.rs`, `queue.rs`, `http_api.rs` (elapsed computation), and `tui.rs` (elapsed display). Low risk but must be done first.
- **Atomic write semantics** — writing `queue_state.toml` on every state transition must use rename-into-place (write to `.smelt-queue-state.toml.tmp`, rename) to prevent partial writes from corrupting the state file. Covered by the existing `atomic-write` pattern in the codebase.
- **Race between write and crash** — if the daemon crashes between dispatching a job and writing the state file, the job may be double-dispatched on restart. This is acceptable (re-queue policy) and should be documented.

## Existing Codebase / Prior Art

- `crates/smelt-cli/src/serve/types.rs` — `QueuedJob`, `JobId`, `JobSource`, `JobStatus`; currently only `Serialize`, no `Deserialize`; `queued_at`/`started_at` use `Instant`
- `crates/smelt-cli/src/serve/queue.rs` — `ServerState` with `VecDeque<QueuedJob>`; needs load-on-startup method
- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig` with `queue_dir: PathBuf`; state file lives at `queue_dir/.smelt-queue-state.toml`
- `crates/smelt-cli/src/commands/serve.rs` — `execute()` that calls `ServerState::new()`; needs to call `ServerState::load_or_new()` instead
- `crates/smelt-cli/src/serve/dispatch.rs` — `dispatch_loop`/`run_job_task`; triggers state transitions that must write state file
- `crates/smelt-core/src/monitor.rs` — `JobMonitor` atomic write pattern (write to `.tmp`, rename) is the precedent to follow

> See `.kata/DECISIONS.md` for all architectural and pattern decisions — D034 (state file TOML format), D100 (atomic file-move semantics) are most relevant.

## Relevant Requirements

- R028 — Persistent queue across `smelt serve` restarts: this milestone fully implements it

## Scope

### In Scope

- `Serialize + Deserialize` on all queue types (`JobId`, `JobSource`, `JobStatus`, `QueuedJob`)
- Replace `Instant` with `SystemTime`/`u64` in `QueuedJob` — update elapsed computation in HTTP API and TUI
- `queue_state.toml` written atomically to `queue_dir/.smelt-queue-state.toml` on every state transition (enqueue, complete, cancel, mark_running)
- `ServerState::load_or_new(queue_dir, max_concurrent)` — loads existing state on startup, re-queues Queued/Retrying/Dispatching/Running jobs (any non-terminal status → Queued)
- Integration test: restart with queued jobs → re-dispatch confirmed
- `examples/server.toml` gets a comment noting queue_dir persistence

### Out of Scope / Non-Goals

- Redis, SQLite, or any external queue service — filesystem only
- In-flight job reattachment (detecting a container still running from a previous daemon instance)
- Job deduplication across restarts — double-dispatch is acceptable
- Volume provisioning — user manages their own Docker volume or PV

## Technical Constraints

- `std::time::Instant` is not serializable and not meaningful across process restarts — must use `std::time::SystemTime` (or `u64` Unix epoch seconds) for all time fields
- Atomic write: always write `.smelt-queue-state.toml.tmp` then `fs::rename()` — never write directly to the target path
- `deny_unknown_fields` should NOT be set on `PersistedJob` (the on-disk form) — future fields should be additive without breaking older state files
- `ServerState::load_or_new()` must tolerate a missing state file (first run) and a corrupted state file (warn and start fresh) — never panic

## Integration Points

- `queue_dir` filesystem — state file written here; must be on a persistent path for durability
- `smelt-core/monitor.rs` — JobMonitor pattern for atomic write (precedent)
- `tui.rs` and `http_api.rs` — both compute elapsed time from `QueuedJob`; need updating for `SystemTime`

## Open Questions

- Should re-queued jobs preserve their `attempt` count from before the restart? **Yes** — a job that failed twice before the restart should still count those attempts toward `retry_attempts`.
- Should the state file be human-readable TOML or compact JSON? **TOML** — matches the existing `.smelt/runs/<job>/state.toml` convention; operator-inspectable without tooling.
