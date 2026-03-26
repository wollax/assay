# Kata State

**Active Milestone:** M010 — Pluggable State Backend
**Active Slice:** S03 — CapabilitySet degradation paths
**Active Task:** none (S02 complete, S03 not started)
**Phase:** Planning

## Recent Decisions
- D156: Arc<dyn StateBackend> in OrchestratorConfig (supersedes D151's Box)
- D157: OrchestratorConfig Default uses placeholder path for LocalFsBackend
- D158: persist_state removed from pub(crate) API after backend wiring
- D159: Feature-gated RunManifest fields require split schema snapshot tests (orchestrate vs non-orchestrate)
- Fixed T01 contract test: checkpoint assertion path corrected from checkpoint.json to checkpoints/latest.md

## Blockers
- None

## Next Action
Begin S03: CapabilitySet degradation paths. Add orchestrator checks for `backend.capabilities().supports_messaging` before mesh routing and `supports_gossip_manifest` before knowledge manifest injection. Create `NoopBackend` test helper. Write `test_mesh_degrades_gracefully_without_messaging` and `test_gossip_degrades_gracefully_without_manifest` tests.
