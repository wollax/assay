# Kata State

**Active Milestone:** M011 — Concrete Remote Backends
**Active Slice:** S04 — SshSyncBackend and CLI/MCP factory wiring
**Phase:** Executing

## Recent Decisions
- D172: GitHubBackend factory dispatch has no env-var gate (unlike LinearBackend)
- D173: SshSyncBackend uses ssh_run() with shell_quote() for remote commands; scp paths use Command::arg()
- D174: SshSyncBackend read_run_state returns Ok(None) on scp pull failure (file not found = normal)

## Blockers
- None

## Progress
- M010 ✅ — Pluggable State Backend complete (all 4 slices, 1488+ tests)
- M011/S01 ✅ — assay-backends crate scaffold and StateBackendConfig variants complete (1499 tests green)
- M011/S02 ✅ — LinearBackend complete (8 contract tests + factory dispatch — 1501 total tests green)
- M011/S03 ✅ — GitHubBackend complete (8 contract tests + factory dispatch — 1501 total tests green)
- M011/S04 🔵 — SshSyncBackend + CLI/MCP factory wiring (T01 ✅, T02 ✅, T03 next)

## Next Action
S04/T03: Wire backend_from_config() into CLI/MCP construction sites
