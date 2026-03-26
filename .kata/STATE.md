# Kata State

**Active Milestone:** M010 — Pluggable State Backend
**Active Slice:** S03 — CapabilitySet degradation paths
**Active Task:** T02 — Capability guards in mesh and gossip executors
**Phase:** Executing

## Recent Decisions
- D156: Arc<dyn StateBackend> in OrchestratorConfig (supersedes D151's Box)
- D157: OrchestratorConfig Default uses placeholder path for LocalFsBackend
- D158: persist_state removed from pub(crate) API after backend wiring
- D159: Feature-gated RunManifest fields require split schema snapshot tests (orchestrate vs non-orchestrate)

## Blockers
- None

## Next Action
Execute S03/T02: Add capability guards to `run_mesh()` and `run_gossip()` that check `backend.capabilities()` before exercising messaging and gossip-manifest features. This will turn T01's red gossip degradation test green.
