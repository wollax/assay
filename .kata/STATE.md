# Kata State

**Active Milestone:** M010 — Pluggable State Backend
**Active Slice:** None — planning complete, ready to execute S01
**Active Task:** None
**Phase:** Planning complete

## Recent Decisions

- D149: StateBackend is a deliberate, scoped exception to D001 (zero-trait convention); sole exception, tightly contained
- D150: StateBackend trait methods are sync; async backends internalize their runtime (preserves D007)
- D151: Box<dyn StateBackend> in OrchestratorConfig, not a generic parameter (avoids viral generics)
- D152: Tier-1/Tier-2 split — heartbeats and per-tick routing are LocalFsBackend internals, not trait methods
- D153: StateBackendConfig as enum with LocalFs + Custom variants

## Blockers

- None

## Next Action

Begin M010/S01: define `StateBackend` trait, `CapabilitySet`, `StateBackendConfig`, and `LocalFsBackend` skeleton in `assay-core`. Write contract tests first (test-first discipline). Read M010-CONTEXT.md and M010-ROADMAP.md S01 entry before starting.
