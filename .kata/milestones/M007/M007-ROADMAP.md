# M007: Persistent Queue

**Vision:** `smelt serve` survives restarts without losing queued work. Queue state is written to `queue_dir/.smelt-queue-state.toml` on every transition and loaded on startup — jobs that were queued or in-flight when the daemon died are automatically re-dispatched, with no re-submission required and no new service dependencies.

## Success Criteria

- Kill `smelt serve` mid-run (with jobs queued), restart it from the same config, and all previously-queued jobs are automatically re-dispatched — no operator intervention required
- `queue_dir/.smelt-queue-state.toml` exists and contains a human-readable TOML snapshot of all job states after any enqueue, complete, or cancel
- Jobs that were `Dispatching` or `Running` at crash time are re-queued on restart (not lost, not permanently Failed)
- Jobs preserve their `attempt` count across restarts — a job that failed twice before the restart still counts those attempts toward `retry_attempts`
- `smelt run manifest.toml` is unchanged — zero regressions in `cargo test --workspace`

## Key Risks / Unknowns

- **`Instant` not serializable** — `QueuedJob.queued_at` / `started_at` use `std::time::Instant`, which is not serializable and not meaningful across process restarts. Must migrate to `SystemTime`/`u64` before any serialization can work. This is the first and blocking change — everything downstream depends on it.
- **Elapsed computation breakage** — `http_api.rs` and `tui.rs` both compute elapsed from `Instant`-based fields. Changing to `SystemTime` requires updating both. Risk of regressions in the live display.

## Proof Strategy

- **`Instant` migration** → retire in S01 by proving all 19 existing serve tests still pass after type change, and HTTP `elapsed_secs` / TUI elapsed column show correct values
- **State file correctness** → retire in S02 by round-trip test: write state, kill process (simulated), reconstruct `ServerState` from file, assert all jobs present with correct fields
- **Restart recovery** → retire in S03 by integration test: enqueue jobs, serialize state, drop `ServerState`, reconstruct from file, call `dispatch_loop` (with mock dispatch), assert jobs re-dispatched

## Verification Classes

- Contract verification: `QueuedJob` serialize/deserialize round-trip; `ServerState::load_or_new()` handles missing file (first run) and corrupted file (warn + fresh start); atomic write uses `.tmp` + rename
- Integration verification: state file written after each transition; restart with queued jobs → re-dispatch confirmed; attempt count preserved
- Operational verification: `kill -9 smelt-serve` mid-queue → restart → jobs re-dispatched; `smelt status <job>` still works for re-dispatched jobs
- UAT / human verification: manual kill-and-restart with 3 real jobs in queue (see M007-UAT.md when written)

## Milestone Definition of Done

This milestone is complete only when all are true:

- `QueuedJob` and all queue types are `Serialize + Deserialize`; `Instant` fields replaced with `u64` Unix epoch seconds
- `queue_dir/.smelt-queue-state.toml` is written atomically on every state transition
- `ServerState::load_or_new()` exists and re-queues all non-terminal jobs on startup
- `commands/serve.rs` calls `load_or_new()` instead of `new()`
- Integration test proves restart-and-redispatch end-to-end
- `cargo test --workspace` all green; all 19 existing serve tests pass

## Requirement Coverage

- Covers: R028 (persistent queue across `smelt serve` restarts)
- Partially covers: none
- Leaves for later: R026 (Linear/GitHub Issues backlog), R027 (SSH workers)
- Orphan risks: none

## Slices

- [x] **S01: Serialize queue types + migrate Instant to SystemTime** `risk:high` `depends:[]`
  > After this: all 19 existing serve tests pass with `SystemTime`-based timing; `QueuedJob` round-trips through TOML; HTTP `elapsed_secs` and TUI elapsed column show correct values.

- [x] **S02: Atomic state file — write on every transition** `risk:medium` `depends:[S01]`
  > After this: every enqueue, complete, cancel, and mark_running writes `queue_dir/.smelt-queue-state.toml` atomically; round-trip unit test proves the file can reconstruct a full `ServerState`; existing serve tests still pass.

- [ ] **S03: Load-on-startup + restart-recovery integration test** `risk:medium` `depends:[S02]`
  > After this: `smelt serve` calls `ServerState::load_or_new()` on startup; integration test proves enqueue → serialize → drop → reconstruct → dispatch cycle; attempt counts preserved; `cargo test --workspace` all green.

## Boundary Map

### S01 → S02, S03

Produces:
- `QueuedJob` with `#[derive(Serialize, Deserialize)]`; `queued_at: u64` (Unix epoch secs), `started_at: Option<u64>` replacing `Instant` fields
- `JobId`, `JobSource`, `JobStatus` all derive `Deserialize` (already have `Serialize`)
- `PersistedJob` struct (or inline `QueuedJob`) as the on-disk TOML representation — **no `deny_unknown_fields`** for forward compatibility
- `elapsed_secs()` helper on `QueuedJob` using `SystemTime::now()` vs stored epoch — used by http_api and tui
- All 19 existing serve tests passing with new time types

Consumes:
- nothing (first slice)

### S02 → S03

Produces:
- `write_queue_state(queue_dir, jobs)` — atomic write to `queue_dir/.smelt-queue-state.toml.tmp` then rename; called after every mutation to `ServerState`
- `read_queue_state(queue_dir) -> Vec<PersistedJob>` — returns empty vec on missing file, logs warn + returns empty on parse error
- Round-trip unit test: `write_queue_state` → `read_queue_state` → assert all fields equal

Consumes from S01:
- `QueuedJob` with `Serialize + Deserialize`

### S03 (final wiring — no new public surfaces)

Produces:
- `ServerState::load_or_new(queue_dir, max_concurrent)` — reads state file, re-queues non-terminal jobs (Queued, Retrying, Dispatching, Running → Queued), respects `attempt` count
- `commands/serve.rs` calls `load_or_new()` instead of `ServerState::new()`
- Integration test: serialize 3 queued jobs → drop state → reconstruct via `load_or_new()` → assert all 3 re-dispatched with correct attempt counts
- `examples/server.toml` updated with persistence note

Consumes from S01:
- `QueuedJob` with `Serialize + Deserialize`

Consumes from S02:
- `write_queue_state`, `read_queue_state`
