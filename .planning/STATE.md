# State

## Current Position

Phase: 1 of 10 — Project Bootstrap & Git Operations Layer
Plan: Establish Rust project, CLI skeleton, `SmeltGitOps` trait, `.smelt/` state directory, CI pipeline
Status: Not started
Progress: ░░░░░░░░░░ 0/10

Last activity: 2026-03-09 — Roadmap created for v0.1.0

## Performance Metrics

| Metric | Value |
|--------|-------|
| Phases completed | 0 |
| Phases remaining | 10 |
| Requirements covered | 0/12 |
| Blockers | 0 |
| Technical debt items | 0 |

## Accumulated Context

### Decisions

- v0.1.0 scope: Orchestration PoC — worktree coordination + merge + AI conflict resolution
- Language: Rust — ecosystem alignment with Assay, single-binary distribution, `tokio` async runtime
- Git operations: Shell-out to `git` CLI behind `SmeltGitOps` trait; `gix` for reads where mature
- Build order: Human fallback before AI resolution (safety net first, optimization second)
- Scripted sessions before real agents (enables full-pipeline testing without AI costs)
- Sequential merge strategy (not octopus) — isolates conflicts to specific branch pairs
- No Assay integration in v0.1.0 — focus on core orchestration loop
- No PR creation, notifications, or cost tracking in v0.1.0

### Blockers

(None)

### Technical Debt

(None)
