---
id: S04
milestone: M008
status: ready
---

# S04: Dispatch routing + round-robin + TUI/API worker field — Context

## Goal

Wire the dispatch loop to route jobs to SSH workers (round-robin with offline-worker skip and re-queue), add `worker_host` to `QueuedJob`, `JobStateResponse`, and the TUI as a 6th column, and validate the full end-to-end path with mock workers.

## Why this Slice

S01–S03 build the SSH primitives and state sync abstractions. S04 is the wiring slice: it plugs those primitives into the existing `dispatch_loop` / `run_job_task` machinery so `smelt serve` actually dispatches to remote workers in production. It also closes the observability gap — `worker_host` is visible in the API and TUI. This slice completes R027 and makes M008 a usable milestone.

## Scope

### In Scope

- Dispatch routing in `dispatch_loop`/`run_job_task`: `if config.workers.is_empty() { local } else { ssh_dispatch }` — no workers = local execution, workers present = SSH dispatch
- Round-robin worker selection: `next_worker_idx` tracked in `ServerState` (wrapping increment per dispatch); probe each candidate; skip offline workers; cycle to next worker on probe failure
- All-workers-offline handling: re-queue the job (set status back to `Queued`), log a WARN, continue the dispatch loop — no fallback to local, no immediate failure
- `worker_host: Option<String>` field added to `QueuedJob` (set at dispatch time, `None` for local jobs) and serialized into queue state
- `worker_host` added to `JobStateResponse` in `http_api.rs` — visible in `GET /api/v1/jobs` and `GET /api/v1/jobs/:id`
- TUI: new 6th column "Worker" between Status and Attempt; truncated to ~15 chars; shows hostname only (not user@host); empty/`-` for local jobs; `local` label when no workers configured is not needed — empty cell is sufficient
- Startup INFO log: "dispatching to N SSH workers" or "local dispatch mode (no workers configured)" emitted once at `smelt serve` startup before the dispatch loop begins
- Unit tests via `MockSshClient`: 2 workers, 4 jobs → assert round-robin assignment; 1st worker offline → all jobs route to 2nd worker; all workers offline → jobs remain Queued
- `cargo test --workspace` all green, zero regressions
- `examples/server.toml` updated if any new config fields are introduced (expected: none in S04)

### Out of Scope

- Weighted or priority-based worker selection (only round-robin in M008)
- Dynamic worker health monitoring between dispatches (probe happens at dispatch time only)
- Worker load reporting or capacity limits
- Fallback to local execution when all workers are offline (re-queue only)
- TUI footer/header showing worker count or mode — INFO log at startup is sufficient
- Cancellation of an in-flight SSH job from the TUI or API (cancellation is Ctrl+C only, same as local)
- `StateBackend` wiring into S04 dispatch (S03 owns that interface; S04 calls `sync_state_back()` after `run_remote_job()` returns, delegating to whatever S03 provides)

## Constraints

- `QueuedJob` gains `worker_host: Option<String>` — must add `#[serde(default)]` so existing persisted queue TOML files (from M007) round-trip without error (backward compat)
- Round-robin index must survive dispatch-loop restarts within a single serve session (stored in `ServerState`, not persisted to disk — re-starts from 0 on process restart, which is acceptable)
- D121 applies: SSH dispatch functions remain generic over `C: SshClient`; `dispatch_loop` holds a `SubprocessSshClient` instance for production and receives a `MockSshClient` in tests
- Probe at dispatch time uses `WorkerConfig` + `ssh_timeout_secs` from `ServerConfig` — same timeout as S01/S02
- TUI column widths: existing columns must not shrink; add Worker column with `Constraint::Length(16)` (15 chars + 1 spacing); if terminal is too narrow the existing `Fill(1)` Manifest column absorbs the squeeze

## Integration Points

### Consumes

- `SshClient::probe()`, `SubprocessSshClient` (S01) — used to check worker reachability at dispatch time
- `deliver_manifest<C>()`, `run_remote_job<C>()` (S02) — called in sequence for each SSH-dispatched job
- `StateBackend::pull()` / `sync_state_back<C>()` (S03) — called after `run_remote_job()` returns; on failure, job marked Failed (per S03 context)
- `ServerState` in `queue.rs` — extended with `next_worker_idx: usize`
- `QueuedJob` in `types.rs` — extended with `worker_host: Option<String>`
- `JobStateResponse` in `http_api.rs` — extended with `worker_host: Option<String>`
- TUI render in `tui.rs` — 6th column added

### Produces

- `ssh_dispatch_job<C: SshClient>()` free function (or inline in `run_job_task`) — orchestrates probe → deliver_manifest → run_remote_job → sync_state_back for a single job+worker pair
- Updated `dispatch_loop` routing: worker-aware dispatch vs local fallback
- `ServerState::next_worker()` method — returns the next `WorkerConfig` by round-robin index, skipping offline workers via probe
- `worker_host` field wired end-to-end: set in `run_job_task`, propagated through `QueuedJob`, surfaced in API and TUI
- Integration tests: round-robin assertion, all-offline re-queue assertion, end-to-end mock test (deliver → exec → sync → Complete)

## Open Questions

- **`ssh_dispatch_job` location** — free function in `dispatch.rs` alongside `run_job_task`, or a new `ssh_dispatch.rs` module? `dispatch.rs` is the natural home since it already owns `run_job_task`; only worth splitting if the function grows large. Decide at planning.
- **Probe-before-dispatch vs probe-on-failure** — current plan probes the candidate worker before attempting delivery; alternatively, attempt delivery directly and treat scp failure as an offline signal. Probe-first is safer (avoids partial delivery) but adds an extra SSH round-trip per job. Probe-first is the planned approach; confirm at planning.
- **TUI column header text** — "Worker" or "Host"? "Worker" matches the config term (`[[workers]]`); "Host" is more literal. Low-stakes — agent decides.
