# Kata State

**Active Milestone:** M007 — Persistent Queue
**Active Slice:** S02 — Atomic state file — write on every transition
**Active Task:** (S02 not yet started)
**Phase:** Planning

## Milestone Plan

**M007 — Persistent Queue** (3 slices)
- [x] S01: Serialize queue types + migrate Instant to SystemTime
- [ ] S02: Atomic state file — write on every transition
- [ ] S03: Load-on-startup + restart-recovery integration test

**M008 — SSH Worker Pools** (4 slices, planned)
- [ ] S01: WorkerConfig + SSH connection proof
- [ ] S02: Manifest delivery + remote smelt run execution
- [ ] S03: State sync back via scp
- [ ] S04: Dispatch routing + round-robin + TUI/API worker field

## Recent Decisions

- D108: Queue persistence uses TOML file in queue_dir, not Redis/SQLite
- D109: In-flight jobs at crash time are re-queued (not Failed) on restart
- D110: Instant fields replaced with u64 Unix epoch seconds for serializability
- D111: SSH dispatch uses subprocess ssh/scp (not openssh/ssh2 crate)
- D112: Worker key_env field holds name of env var with SSH key path

## Blockers

None.

## Next Action

S01 complete. Begin S02: Atomic state file — write on every transition. Plan: implement `write_queue_state(queue_dir, jobs)` (atomic write to `.smelt-queue-state.toml.tmp` then rename) and `read_queue_state(queue_dir) -> Vec<QueuedJob>` (empty vec on missing file, warn + empty on parse error); wire calls into every `ServerState` mutation; add round-trip unit test.
