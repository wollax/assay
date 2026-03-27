# Kata State

**Active Milestone:** M012 — Checkpoint Persistence on Remote Backends
**Active Slice:** S01 — GuardDaemon backend plumbing and contract tests
**Active Task:** none (planning complete, ready to execute)
**Phase:** Planning

## Recent Decisions
- D175: GuardDaemon accepts Arc<dyn StateBackend>; CLI defaults to LocalFsBackend
- D176: save_checkpoint_summary called synchronously inside async GuardDaemon run loop — accepted risk

## Blockers
- None

## Progress
- M011 ✅ COMPLETE (R076–R079 validated, 1526 tests with all features)
- M012/S01 — ready to execute

## Next Action
Execute M012/S01: Add `backend: Arc<dyn StateBackend>` to GuardDaemon, update `start_guard()` signature, wire CLI, write contract tests, run `just ready`.
