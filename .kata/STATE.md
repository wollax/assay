# Kata State

**Active Milestone:** M008 — SSH Worker Pools (planned)
**Active Slice:** None (M007 complete)
**Active Task:** None
**Phase:** M007 Complete — Ready for M008

## Milestone Plan

**M007 — Persistent Queue** (3 slices) ✅ COMPLETE
- [x] S01: Serialize queue types + migrate Instant to SystemTime
- [x] S02: Atomic state file — write on every transition
- [x] S03: Load-on-startup + restart-recovery integration test

**M008 — SSH Worker Pools** (4 slices, planned)
- [ ] S01: WorkerConfig + SSH connection proof
- [ ] S02: Manifest delivery + remote smelt run execution
- [ ] S03: State sync back via scp
- [ ] S04: Dispatch routing + round-robin + TUI/API worker field

## Recent Decisions

- D120: `load_or_new(queue_dir, max_concurrent)` — always enables persistence; remaps Dispatching/Running → Queued; delegates to new_with_persistence
- D116: try_dispatch does NOT write state — Dispatching is transient
- D115: new() unchanged (queue_dir: None); new_with_persistence() added alongside
- D109: In-flight jobs at crash time are re-queued (not Failed) on restart
- D108: Queue persistence uses TOML file in queue_dir, not Redis/SQLite

## Blockers

None.

## Next Action

M007 is complete. R028 validated. Begin M007 milestone summary or start M008 planning.
