# Kata State

**Active Milestone:** M012 — Tracker-Driven Autonomous Dispatch
**Active Slice:** S03 — GitHub Issues Tracker Backend (next)
**Active Task:** None — starting S03
**Phase:** Planning

## Progress
- [x] S01: M011 Leftover Cleanup — Tracing Migration & Flaky Test Fix
- [x] S02: TrackerSource Trait, Config, & Template Manifest
- [ ] S03: GitHub Issues Tracker Backend `depends:[S02]`
- [ ] S04: Linear Tracker Backend `depends:[S02]`
- [ ] S05: Dispatch Integration, State Backend Passthrough & Final Assembly `depends:[S03,S04]`

## Recent Decisions
- D160: state_backend added to JobManifest in S02 (not deferred to S05)
- D161: issue_to_manifest() is a free function, not a trait method
- D162: Template manifest must have zero [[session]] entries
- D163: StateBackendConfig uses toml::Value for Custom variant
- D158: Bare-message tracing format for default subscriber (S01)

## Blockers
- None

## Next Action
Begin S03: GithubTrackerSource — polls GitHub Issues via `gh` CLI, transitions labels, generates manifests from templates. Proven by unit tests with mock `gh` and integration tests against a real repo (gated by env var).
