# Kata State

**Active Milestone:** M010 — Pluggable State Backend
**Active Slice:** S04 — smelt-agent plugin
**Active Task:** — (not yet started)
**Phase:** Planning

## Recent Decisions
- D160: NoopBackend silently succeeds for all methods even when capabilities are false — test helper, not production backend
- D161: Capability guard pattern — capture bool before thread::scope, guard all feature-specific write sites by value

## Blockers
- None

## Next Action
Implement S04: create `plugins/smelt-agent/` directory with AGENTS.md and three skills (run-dispatch.md, backend-status.md, peer-message.md).
