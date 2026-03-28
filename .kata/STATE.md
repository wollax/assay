# Kata State

**Active Milestone:** M013 — Tech Debt & Deferred Features
**Active Slice:** S03 — OTel metrics
**Active Task:** T03 — Instrument pipeline and merge code paths
**Phase:** Executing

## Recent Decisions
- D185: OTel metrics verification strategy — contract tests with in-process MeterProvider, real collector is UAT
- D186: Recording functions unconditional (no cfg guards at call sites) — feature gating internal to functions
- D182: Orphan spans treated as additional roots at depth 0 in flatten_span_tree()
- D183: TraceViewer loads traces on screen transition (t key), not on every draw
- D184: Two-mode screen pattern — selected_trace Option<usize> switches list/detail mode

## Blockers
- None

## Progress
- M012 ✅ COMPLETE (R080 validated, 1503 tests with all features)
- M013:
  - S01 ✅ complete (R081 validated, 1501 tests)
  - S02 ✅ complete (R066 validated, 7 integration tests, `just ready` green)
  - S03: OTel metrics — T01 done, T02 done (metrics infra), T03 next
  - S04: Wizard runnable criteria — not started

## Next Action
Execute T03: Instrument pipeline and merge code paths with the five recording call sites.
