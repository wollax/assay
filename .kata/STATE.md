# Kata State

**Active Milestone:** M008 — SSH Worker Pools
**Active Slice:** S03 — State sync back via scp
**Phase:** Ready to plan

## Milestone Plan

**M007 — Persistent Queue** ✅ COMPLETE
- [x] S01: Serialize queue types + migrate Instant to SystemTime
- [x] S02: Atomic state file — write on every transition
- [x] S03: Load-on-startup + restart-recovery integration test

**M008 — SSH Worker Pools** (4 slices, in progress)
- [x] S01: WorkerConfig + SSH connection proof ✅
- [x] S02: Manifest delivery + remote smelt run execution ✅
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

Plan and execute S03: State sync back via scp — after `smelt run` completes on the worker, scp `.smelt/runs/<job>/` back to the dispatcher.
