# Kata State

**Active Milestone:** M013 — Tech Debt & Deferred Features
**Active Slice:** S02 — TUI Trace Viewer
**Active Task:** T03 — Render trace list and span tree
**Phase:** Executing

## Recent Decisions
- D178: Wizard cmd field is optional and per-criterion; empty input skips cmd
- D179: OTel MeterProvider stored in TracingGuard alongside SdkTracerProvider
- D180: TUI trace viewer reads top-20 most-recent trace files sorted by mtime
- D181: GhRunner::gh_error consolidates warn + error construction for all gh CLI failures

## Blockers
- None

## Progress
- M012 ✅ COMPLETE (R080 validated, 1529 tests with all features)
- M013: S01 ✅ complete (R081 validated, 1501 tests)
  - S02: TUI trace viewer — T01 ✅, T02 ✅ (two-mode navigation, span tree drill-down, load_trace_spans)
  - S03: OTel metrics
  - S04: Wizard runnable criteria

## Next Action
Execute M013/S02/T03: Render trace list and span tree. Note: T02 already implemented draw_trace_viewer with List-based rendering for both views — T03 may focus on polish, final integration test pass, and `just ready`.
