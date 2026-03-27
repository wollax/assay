# Kata State

**Active Milestone:** M011 — Concrete Remote Backends
**Active Slice:** S01 — assay-backends crate scaffold and StateBackendConfig variants
**Active Task:** none (planning complete, S01 not started)
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

## Next Action
Execute M011/S01: create `crates/assay-backends/` crate, add Linear/GitHub/Ssh variants to `StateBackendConfig`, regenerate schema snapshots, implement `backend_from_config()` stub factory, run `just ready`.
