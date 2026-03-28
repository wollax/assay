# Kata State

**Active Milestone:** M012 — Tracker-Driven Autonomous Dispatch
**Active Slice:** S04 — Linear Tracker Backend (next)
**Active Task:** None
**Phase:** Executing

## Progress
- [x] S01: M011 Leftover Cleanup — Tracing Migration & Flaky Test Fix
- [x] S02: TrackerSource Trait, Config, & Template Manifest
- [x] S03: GitHub Issues Tracker Backend
- [ ] S04: Linear Tracker Backend `depends:[S02]`
- [ ] S05: Dispatch Integration, State Backend Passthrough & Final Assembly `depends:[S03,S04]`

## Recent Decisions
- D164: GhClient trait with generic parameter (mirrors SshClient/D121)
- D165: TrackerConfig.repo required for GitHub, ignored for Linear
- D166: edit_labels combines add+remove in single gh issue edit call
- D160: state_backend added to JobManifest in S02

## Blockers
- None

## Next Action
Begin S04: LinearTrackerSource via GraphQL API (LINEAR_API_KEY), mirrors GhClient/GithubTrackerSource pattern.
