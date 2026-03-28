# Kata State

**Active Milestone:** M012 — Tracker-Driven Autonomous Dispatch
**Active Slice:** S05 — Dispatch Integration, State Backend Passthrough & Final Assembly
**Active Task:** None — starting S05
**Phase:** Planning

## Progress
- [x] S01: M011 Leftover Cleanup — Tracing Migration & Flaky Test Fix
- [x] S02: TrackerSource Trait, Config, & Template Manifest
- [x] S03: GitHub Issues Tracker Backend
- [x] S04: Linear Tracker Backend
  - [x] T01: LinearClient trait, ReqwestLinearClient, and MockLinearClient
  - [x] T02: LinearTrackerSource bridging LinearClient to TrackerSource
  - [x] T03: TrackerConfig Linear fields and validation
- [ ] S05: Dispatch Integration, State Backend Passthrough & Final Assembly `depends:[S03,S04]`

## Recent Decisions
- D167: Linear issue UUID as TrackerIssue.id (not human-readable identifier)
- D168: reqwest promoted to production dep in smelt-cli
- D169: Label UUID caching in ensure_labels() HashMap
- D170: Linear transition_state uses two separate mutations (remove + add)

## Blockers
- None

## Next Action
Advance to S05: wire TrackerPoller into dispatch_loop, state_backend passthrough into AssayInvoker, TUI tracker column, server.toml example update, README update. Call ensure_labels() at poller startup before first poll cycle.
