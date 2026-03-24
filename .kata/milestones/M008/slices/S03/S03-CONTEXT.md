---
id: S03
milestone: M008
status: collapsed
---

# S03: State sync back via scp — Context

> **⚠️ COLLAPSED INTO S04**
>
> During the discuss phase, the user decided that state sync is simple enough to implement
> alongside the dispatch routing in S04. S03 as a standalone slice is removed from the plan.
> The planning agent should merge S03's work into S04's task decomposition.

## Decision Rationale

State sync (`scp -r .smelt/runs/<job>/` back to dispatcher) is a small operation — one subprocess call with a warn-on-failure contract. Doing it as a standalone slice creates overhead without proportional value. S04 already owns the dispatch routing layer that triggers this call; it's cleaner to implement `sync_state_back()` there alongside `run_remote_job()`.

Additionally, the user flagged that the state persistence model may change (potential pivot to Linear or other external trackers). This makes it premature to design the state-landing path (`smelt status` integration) in isolation before S04 has full context on the dispatch flow.

## What S04 Should Include From S03

The S04 planner should include these items as tasks within the S04 slice:

- `sync_state_back(worker, job_id, local_base_dir) -> Result<()>` — scps `.smelt/runs/<manifest.job.name>/` from worker to dispatcher; warns on failure; does NOT block job completion on scp error
- Decision on state destination path: syncs to `.smelt/runs/<manifest.job.name>/` on dispatcher (same path that `smelt status <job-name>` reads via `JobMonitor::read()`)
- Accepted limitation: two concurrent jobs with the same manifest `job.name` on the same worker will overwrite each other's state directory. This is a documented constraint; operators must use unique manifest job names for concurrent dispatch.
- `smelt status <manifest-job-name>` works on the dispatcher for synced remote jobs — reads `.smelt/runs/<name>/state.toml` that was synced from the worker
- Sync triggered after `run_remote_job()` returns (regardless of exit code) — state contains the terminal phase information

## Scope (Original, Now Part of S04)

### In Scope (→ S04)

- `sync_state_back()` implementation via `scp -r` subprocess
- Integration test: create a directory with a mock `state.toml` on localhost, scp it to localhost dispatcher path, assert the file arrived
- scp failure is non-fatal: log `warn!`, job stays completed/failed based on `run_remote_job` exit code
- `smelt status <name>` on dispatcher reads the synced state

### Out of Scope

- Streaming state updates during job execution — state is synced once, after completion
- State migration or translation between job_id and manifest job name
- Any external tracker integration (Linear, GitHub Issues) — future milestone

## Open Questions Deferred to S04

- **State collision policy**: if two jobs with the same manifest job name run concurrently on the same worker, their `.smelt/runs/<name>/` directories overwrite each other. S04 should document this explicitly and decide whether to warn the operator at dispatch time.
- **Future-proofing**: if state persistence pivots to an external tracker (Linear), the `sync_state_back` function becomes a no-op or is replaced. The function boundary should be designed to make this swap easy — a single caller in the dispatch path, not spread across multiple components.
