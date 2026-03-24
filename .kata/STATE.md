# Kata State

**Active Milestone:** M008 — SSH Worker Pools
**Active Slice:** S02 — Manifest delivery + remote smelt run execution
**Active Task:** None — planning S02
**Phase:** Planning

## Milestone Plan

**M007 — Persistent Queue** ✅ COMPLETE
- [x] S01: Serialize queue types + migrate Instant to SystemTime
- [x] S02: Atomic state file — write on every transition
- [x] S03: Load-on-startup + restart-recovery integration test

**M008 — SSH Worker Pools** (4 slices, in progress)
- [x] S01: WorkerConfig + SSH connection proof ✅
- [ ] S02: Manifest delivery + remote smelt run execution
- [ ] S03: State sync back via scp
- [ ] S04: Dispatch routing + round-robin + TUI/API worker field

## Recent Decisions

- D121: SshClient uses generic `<C: SshClient>` at callsites, not `dyn SshClient` — RPITIT async fn not object-safe; consistent with D060
- D120: `load_or_new(queue_dir, max_concurrent)` — always enables persistence; remaps Dispatching/Running → Queued
- D111: SSH dispatch uses subprocess ssh/scp, not openssh/ssh2 crate
- D112: WorkerConfig uses `key_env` (env var name) not key value directly
- D017: deny_unknown_fields on WorkerConfig

## Blockers

None.

## Next Action

S01 complete ✅. Start S02 planning: decompose "Manifest delivery + remote smelt run execution" into tasks — deliver_manifest() via scp, run_remote_job() via ssh, integration test with localhost SSH.
