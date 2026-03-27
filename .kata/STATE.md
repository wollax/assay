# Kata State

**Active Milestone:** M011 — Concrete Remote Backends
**Active Slice:** S03 — GitHubBackend
**Active Task:** T02 ✅
**Phase:** Executing

## Recent Decisions
- D160: assay-backends as new leaf crate (linear/github/ssh feature flags)
- D164: LinearBackend capabilities — messaging=false, annotations=true, checkpoints=false
- D165: backend_from_config factory fn in assay_backends::factory
- D168: LinearBackend uses reqwest::blocking, not scoped async runtime (supersedes D161)
- D169: backend_from_config graceful fallback when LINEAR_API_KEY missing
- D170: GitHubBackend capabilities all-false (CapabilitySet::none())
- D171: GitHubBackend uses --body-file - with stdin pipe for body content
- D172: GitHubBackend factory dispatch has no env-var gate (unlike LinearBackend)

## Blockers
- None

## Progress
- M010 ✅ — Pluggable State Backend complete (all 4 slices, 1488+ tests)
- M011/S01 ✅ — assay-backends crate scaffold and StateBackendConfig variants complete (1499 tests green)
- M011/S02 ✅ — LinearBackend complete (8 contract tests + factory dispatch — 1501 total tests green)
- M011/S03 ✅ — GitHubBackend complete (T01 ✅ contract tests, T02 ✅ implementation + factory wiring — all 8 contract tests + factory tests green)
- M011/S04 ⬜ — SshSyncBackend + CLI/MCP factory wiring

## Next Action
S04: SshSyncBackend implementation + CLI/MCP factory wiring
