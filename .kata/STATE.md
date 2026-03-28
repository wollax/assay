# Kata State

**Active Milestone:** M012 — Tracker-Driven Autonomous Dispatch ✅ Complete
**Active Slice:** None — all slices complete
**Active Task:** None
**Phase:** Complete

## Progress
- [x] S01: M011 Leftover Cleanup — Tracing Migration & Flaky Test Fix
- [x] S02: TrackerSource Trait, Config, & Template Manifest
- [x] S03: GitHub Issues Tracker Backend
- [x] S04: Linear Tracker Backend
- [x] S05: Dispatch Integration, State Backend Passthrough & Final Assembly
  - [x] T01: State backend passthrough in AssayInvoker
  - [x] T02: TrackerPoller struct and AnyTrackerSource enum
  - [x] T03: Wire TrackerPoller into serve execute(), TUI Source column, and docs

## Recent Decisions
- D171: AnyTrackerSource enum for non-object-safe TrackerSource dispatch
- D172: TrackerPoller poll errors are non-fatal (log + continue)
- D173: TrackerPoller uses std::future::pending() placeholder when no tracker configured

## Blockers
- None

## Milestone Summary
M012 is complete. `smelt serve` now autonomously polls GitHub Issues and Linear for work items labeled `smelt:ready`, generates manifests from templates, transitions labels through the lifecycle, enqueues into the dispatch pipeline, and displays tracker-sourced jobs in the TUI. `state_backend` in JobManifest is forwarded into Assay RunManifest TOML (R075 validated). 398 workspace tests pass. Live end-to-end UAT with real `gh` CLI and Linear API deferred per S05-UAT.md.

## Next Action
Start M013 planning or file a new milestone. Live UAT for M012 can be done independently against real GitHub/Linear credentials following S05-UAT.md.
