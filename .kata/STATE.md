# Kata State

**Active Milestone:** M010 — Pluggable State Backend
**Active Slice:** S01 — StateBackend trait and CapabilitySet
**Active Task:** T01 — Define StateBackendConfig enum in assay-types with schema snapshot
**Phase:** Planning → Executing

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
Execute T01: create `crates/assay-types/src/state_backend.rs` with `StateBackendConfig` enum, register schema, add snapshot test, lock with `cargo insta review`.
