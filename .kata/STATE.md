# Kata State

**Active Milestone:** M010 — Pluggable State Backend
**Active Slice:** S03 — CapabilitySet degradation paths
**Active Task:** none (S02 complete, S03 not started)
**Phase:** Executing

## Recent Decisions
- D156: Arc<dyn StateBackend> in OrchestratorConfig (supersedes D151's Box)
- D157: OrchestratorConfig Default uses placeholder path for LocalFsBackend
- D158: persist_state removed from pub(crate) API after backend wiring
- D159: Feature-gated RunManifest fields require split schema snapshot tests (orchestrate vs non-orchestrate)

## Blockers
- None

## Next Action
Execute S03/T01: Create NoopBackend test helper and write degradation integration tests (red state). Then T02: Add capability guards to mesh.rs and gossip.rs to turn tests green.
