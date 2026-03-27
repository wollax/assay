# Kata State

**Active Milestone:** M013 — Tech Debt & Deferred Features
**Active Slice:** S01 — GitHubBackend correctness fixes (Q001–Q004)
**Active Task:** T01 — Write contract tests for Q001–Q004
**Phase:** Executing

## Recent Decisions
- D177: GitHubBackend repo validation is warn-not-error at construction
- D178: Wizard cmd field is optional and per-criterion; empty input skips cmd
- D179: OTel MeterProvider stored in TracingGuard alongside SdkTracerProvider
- D180: TUI trace viewer reads top-20 most-recent trace files sorted by mtime

## Blockers
- None

## Progress
- M012 ✅ COMPLETE (R080 validated, 1529 tests with all features)
- M013: planning complete — 4 slices, all independent

## Next Action
Begin M013/S01: GitHubBackend correctness fixes (Q001–Q004). Create branch `kata/M013/S01`, implement validation warn + issue-0 rejection + GhRunner error helper + factory doc cleanup, write contract tests, `just ready` green.
