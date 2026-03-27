# Kata State

**Active Milestone:** M011 — Concrete Remote Backends
**Active Slice:** S02 — LinearBackend
**Active Task:** Complete
**Phase:** Verifying

## Recent Decisions
- D160: assay-backends as new leaf crate (linear/github/ssh feature flags)
- D161: reqwest async wrapped in scoped new_current_thread runtime per method (superseded by D168)
- D164: LinearBackend capabilities — messaging=false, annotations=true, checkpoints=false
- D165: backend_from_config factory fn in assay_backends::factory
- D168: LinearBackend uses reqwest::blocking, not scoped async runtime (supersedes D161)
- D169: backend_from_config graceful fallback when LINEAR_API_KEY missing

## Blockers
- None

## Progress
- M010 ✅ — Pluggable State Backend complete (all 4 slices, 1488+ tests)
- M011/S01 ✅ — assay-backends crate scaffold and StateBackendConfig variants complete (1499 tests green)
- M011/S02 ✅ — LinearBackend complete (T01 contract tests, T02 implementation — all 8 tests pass, 1499 total)

## Next Action
S02 complete — all tasks done, all slice verification checks pass, `just ready` green with 1499 tests.
