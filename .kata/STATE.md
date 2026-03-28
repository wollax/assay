# Kata State

**Active Milestone:** M013 — Tech Debt & Deferred Features
**Active Slice:** S04 — Wizard runnable criteria
**Active Task:** None — S04 not started
**Phase:** Planning

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
  - S03 ✅ complete (R067 validated — OTel metrics infra + 5 instrumentation sites, 1516 tests, `just ready` green)
  - S04: Wizard runnable criteria — not started

## Next Action
Begin S04: wizard runnable criteria (R082). Touches wizard.rs, spec.rs, create_spec_from_params.
