# Kata State

**Active Milestone:** M007 — Persistent Queue
**Active Slice:** S03 — Load-on-startup + restart-recovery integration test
**Active Task:** None — S03 planning next
**Phase:** Planning S03

## Milestone Plan

**M007 — Persistent Queue** (3 slices)
- [x] S01: Serialize queue types + migrate Instant to SystemTime
- [x] S02: Atomic state file — write on every transition
- [ ] S03: Load-on-startup + restart-recovery integration test

**M008 — SSH Worker Pools** (4 slices, planned)
- [ ] S01: WorkerConfig + SSH connection proof
- [ ] S02: Manifest delivery + remote smelt run execution
- [ ] S03: State sync back via scp
- [ ] S04: Dispatch routing + round-robin + TUI/API worker field

## Recent Decisions

- D114: QueueState wrapper in queue.rs — TOML needs a top-level table; maps to [[jobs]] array-of-tables
- D115: new() unchanged (queue_dir: None); new_with_persistence() added alongside
- D116: try_dispatch does NOT write state — Dispatching is transient
- D108: Queue persistence uses TOML file in queue_dir, not Redis/SQLite
- D109: In-flight jobs at crash time are re-queued (not Failed) on restart

## Blockers

None.

## Next Action

Begin S03: `ServerState::load_or_new(queue_dir, max_concurrent)` — reads state file via `read_queue_state`, remaps non-terminal jobs (Queued/Retrying/Dispatching/Running) to Queued, preserves attempt counts, calls `new_with_persistence`. Wire into `commands/serve.rs`. Add integration test: serialize 3 queued jobs → drop state → reconstruct via `load_or_new()` → assert all 3 re-dispatched with correct attempt counts. `cargo test --workspace` all green.
