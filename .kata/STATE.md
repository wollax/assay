# Kata State

**Active Milestone:** M011 — Concrete Remote Backends (COMPLETE)
**Active Slice:** none
**Phase:** Done

## Recent Decisions
- D173: SshSyncBackend uses ssh_run() with shell_quote() for remote commands; scp paths use Command::arg()
- D174: SshSyncBackend read_run_state returns Ok(None) on scp pull failure (file not found = normal)

## Blockers
- None

## Progress
- M010 ✅ — Pluggable State Backend complete (all 4 slices, 1488+ tests)
- M011/S01 ✅ — assay-backends crate scaffold and StateBackendConfig variants (1499 tests)
- M011/S02 ✅ — LinearBackend: 8 contract tests + factory dispatch (1499 tests)
- M011/S03 ✅ — GitHubBackend: 8 contract tests + factory dispatch (1499 tests)
- M011/S04 ✅ — SshSyncBackend + CLI/MCP factory wiring: 9 contract tests, all 6 callsites use backend_from_config() (1499 tests)
- **M011 ✅ COMPLETE** — all 4 slices done; just ready green with 1499 tests; R076–R079 validated; D160–D174 documented

## Milestone M011 Definition of Done — Verified
- [x] `assay-backends` crate exists, builds clean, listed in workspace members
- [x] `StateBackendConfig` has `Linear`, `GitHub`, `Ssh` variants; schema snapshots committed and green
- [x] `LinearBackend` (feature: linear), `GitHubBackend` (feature: github), `SshSyncBackend` (feature: ssh) all implement `StateBackend` with contract tests passing
- [x] `backend_from_config()` resolves all four variants to `Arc<dyn StateBackend>`
- [x] `assay-cli` and `assay-mcp` use `backend_from_config()` at all construction sites; no hardcoded `LocalFsBackend::new(...)` at manifest-dispatch sites
- [x] `just ready` green with 1499 tests — zero regression
- [x] D160–D174 decisions documented in DECISIONS.md

## Next Action
M011-SUMMARY.md written and committed. M011 complete. Await user direction for M012.
