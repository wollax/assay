# Kata State

**Active Milestone:** M012 — Checkpoint Persistence on Remote Backends
**Active Slice:** S01 — GuardDaemon backend plumbing and contract tests
**Active Task:** T03 — Wire CLI handle_guard_start and run just ready
**Phase:** Executing

## Recent Decisions
- D175: GuardDaemon accepts Arc<dyn StateBackend>; CLI defaults to LocalFsBackend
- D176: save_checkpoint_summary called synchronously inside async GuardDaemon run loop — accepted risk

## Blockers
- None

## Progress
- M011 ✅ COMPLETE (R076–R079 validated, 1526 tests with all features)
- M012/S01/T01 ✅ SpyBackend + contract tests (red state)
- M012/S01/T02 ✅ Backend field, dual constructors, checkpoint routing, start_guard dual sigs — 11 tests green (9 existing + 2 contract)

## Next Action
Execute T03: Wire CLI handle_guard_start with LocalFsBackend and run just ready.
