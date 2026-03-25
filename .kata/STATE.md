# Kata State

**Active Milestone:** M009 — Observability
**Active Slice:** S02 — Pipeline span instrumentation
**Active Task:** T02 — Instrument pipeline functions with #[instrument] and stage-level spans
**Phase:** Executing
**Last Updated:** 2026-03-24
**Requirements Status:** 6 active (R027, R061–R065) · 56 validated · 3 deferred (R025, R066, R067) · 4 out of scope
**Test Count:** 1400+ (all workspace tests pass)

## M009 Progress

5 slices planned:
- [x] S01: Structured tracing foundation and eprintln migration — R060 validated
- [ ] S02: Pipeline span instrumentation — R061 (executing, T01 done, T02 next)
- [ ] S03: Orchestration span instrumentation — R062
- [ ] S04: JSON file trace export and CLI — R063
- [ ] S05: OTLP export and trace context propagation — R064, R065

## Recent Decisions

- D135: tracing-test for span assertion in tests (tracing-test = "0.2" as workspace dev-dep)
- D132: CLI default tracing level is `info`, MCP is `warn`
- D133: Interactive eprint! prompts preserved, not migrated to tracing
- D134: tracing-subscriber added to assay-core for init_tracing()

## Blockers

None.

## Next Action

Execute T02: Instrument pipeline functions with #[instrument] and stage-level spans — make the 4 red-state T01 tests pass.
