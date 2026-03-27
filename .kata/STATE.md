# Kata State

**Active Milestone:** M011 — Concrete Remote Backends
**Active Slice:** S02 — LinearBackend
**Active Task:** —
**Phase:** Planning

## Recent Decisions
- D160: assay-backends as new leaf crate (linear/github/ssh feature flags)
- D161: reqwest async wrapped in scoped new_current_thread runtime per method
- D162: GitHubBackend uses gh CLI for all operations
- D163: SshSyncBackend uses scp via Command::arg() chaining
- D164: LinearBackend capabilities — messaging=false, annotations=true, checkpoints=false
- D165: backend_from_config factory fn in assay_backends::factory

## Blockers
- None

## Progress
- S01 ✅ — assay-backends crate scaffold and StateBackendConfig variants complete (1497 tests green)
  - S01/T01 ✅ — Added Linear/GitHub/Ssh variants to StateBackendConfig, created assay-backends crate
  - S01/T02 ✅ — Added serde round-trip + factory dispatch tests, regenerated schema snapshots, just ready green

## Next Action
S02: Implement LinearBackend with reqwest async wrapped in scoped new_current_thread runtime; mock HTTP contract tests; update backend_from_config() Linear arm.
