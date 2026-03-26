# Kata State

**Active Milestone:** M010 — Pluggable State Backend
**Active Slice:** S02 — LocalFsBackend implementation and orchestrator wiring
**Active Task:** T01 — Add RunManifest.state_backend field and write integration test contracts
**Phase:** Executing

## Recent Decisions
- D156: Arc<dyn StateBackend> in OrchestratorConfig (supersedes D151's Box)
- D157: OrchestratorConfig Default uses placeholder path for LocalFsBackend
- D158: persist_state removed from pub(crate) API after backend wiring

## Blockers
- None

## Next Action
Execute T01: Add `state_backend` field to `RunManifest`, update schema snapshot, write integration test contracts for `LocalFsBackend` real method bodies.
