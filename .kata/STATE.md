# Kata State

**Active Milestone:** M012 — Tracker-Driven Autonomous Dispatch
**Active Slice:** S02 — TrackerSource Trait, Config, & Template Manifest
**Active Task:** None — starting S02
**Phase:** Planning

## Recent Decisions
- D150: Periodic polling, not webhooks
- D151: One tracker source per serve instance
- D152: Template manifest + issue injection
- D153: Label-based lifecycle state machine
- D154: State backend passthrough via Smelt-side serde struct
- D155: GitHub tracker uses `gh` CLI
- D156: Linear tracker uses reqwest::blocking
- D157: Double-dispatch prevention via atomic label transition
- D158: Bare-message format for default tracing subscriber (extends D107)
- D159: Structured fields in tracing warn! calls for teardown paths

## Blockers
- None

## Next Action
S01 complete (all 3 tasks done, all 5 verification checks green; R061 and R062 validated). Advance to S02: TrackerSource Trait, Config, & Template Manifest.
