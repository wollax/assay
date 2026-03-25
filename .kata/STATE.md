# Kata State

**Active Milestone:** M010 — HTTP API Authentication & Code Quality
**Active Slice:** S02 — Teardown error handling + SSH DRY cleanup
**Active Task:** —
**Phase:** Planning

## Recent Decisions
- D132: Bearer token auth for HTTP API via env var names in server.toml
- D133: Read/write permission split — two token levels
- D134: Auth is opt-in (off by default) for backward compatibility
- D135: Auth middleware uses `Option<ResolvedAuth>` as state, always applied
- D136: `write_token_env` required, `read_token_env` optional in `[auth]`
- D137: GET/HEAD = read, all other methods = write
- D138: ResolvedAuth fields pub(crate) for direct test construction

## Blockers
- None

## Next Action
Plan and execute S02 (teardown error handling + SSH DRY cleanup). S02 is independent of S01.
