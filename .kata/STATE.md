# Kata State

**Active Milestone:** M012 — Checkpoint Persistence on Remote Backends
**Active Slice:** S01 — GuardDaemon backend plumbing and contract tests
**Active Task:** T01 — Create SpyBackend and red-state contract tests
**Phase:** Executing

## Recent Decisions
- D175: GuardDaemon accepts Arc<dyn StateBackend>; CLI defaults to LocalFsBackend
- D176: save_checkpoint_summary called synchronously inside async GuardDaemon run loop — accepted risk

## Blockers
- None

## Progress
- M011 ✅ COMPLETE (R076–R079 validated, 1526 tests with all features)
- M012/S01 — planned (3 tasks: T01 SpyBackend+contract tests, T02 backend field+routing, T03 CLI wiring+just ready)

## Next Action
Execute T01: Create SpyBackend test helper and red-state contract tests in daemon.rs.
