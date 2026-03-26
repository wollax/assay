# Kata State

**Active Milestone:** M010 — Pluggable State Backend
**Active Slice:** S02 — LocalFsBackend implementation and orchestrator wiring
**Active Task:** (planning)
**Phase:** Executing

## Recent Decisions
- D149: StateBackend is the sole exception to D001 zero-trait convention
- D150: StateBackend trait methods are sync; async backends internalize their runtime
- D151: Box<dyn StateBackend> in OrchestratorConfig (not generic)
- D152: Tier-1/Tier-2 split — heartbeats stay as LocalFsBackend internals
- D153: StateBackendConfig enum with LocalFs + Custom variants in assay-types
- D154: state_backend module gated behind `orchestrate` feature in assay-core
- D155: Object-safety compile guard `fn _assert_object_safe` in trait module

## Blockers
- None

## Next Action
S01 complete. PR created for `kata/root/M010/S01`. Begin S02: wire `LocalFsBackend` into `OrchestratorConfig`, implement real method bodies, add `RunManifest.state_backend` field, prove backward-compat round-trip.
