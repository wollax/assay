# Kata State

**Active Milestone:** M010 — Pluggable State Backend
**Active Slice:** S01 — StateBackend trait and CapabilitySet
**Active Task:** T02 — Define StateBackend trait, CapabilitySet, and LocalFsBackend skeleton in assay-core
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
Execute T02: create `crates/assay-core/src/state_backend.rs` with `StateBackend` trait, `CapabilitySet`, and `LocalFsBackend` skeleton. Add 6 contract tests in `crates/assay-core/tests/state_backend.rs`. Run `cargo test -p assay-core state_backend` and `cargo test --workspace`.
