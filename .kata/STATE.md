# Kata State

**Active Milestone:** M006 — Parallel Dispatch Daemon
**Active Slice:** S03 — Ratatui TUI + Server Config + Graceful Shutdown
**Phase:** S02 complete, S03 next

## Recent Decisions
- D105: HTTP POST persists TOML body via std::mem::forget(TempPath) — decouples file lifetime from handler scope
- D104: ServerState::complete() sets Retrying in-place (not re-enqueue) — single entry per job in VecDeque
- D103: JobId uses atomic u64 counter for deterministic test IDs
- D102: ServerConfig is a separate TOML file, not embedded in job manifests
- D101: axum for HTTP API, ratatui + crossterm for TUI
- D100: Queue pickup via file-move (queue_dir → queue_dir/dispatched/) — atomic, restart-safe
- D099: CancellationToken (tokio-util) for broadcast cancellation across N job tasks
- D098: In-process tokio tasks (not subprocess) for job execution

## Completed Milestones
- M001: ✅ Docker-First Infrastructure MVP (2026-03-17)
- M002: ✅ Real Assay Integration (2026-03-17)
- M003: ✅ GitHub Forge + PR Lifecycle (2026-03-21)
- M004: ✅ Docker Compose Runtime (2026-03-23)
- M005: ✅ Kubernetes Runtime (2026-03-23, pending live UAT)

## M006 Slices
- S01: [skipped — S02 absorbed all foundational work]
- S02: ✅ Directory Watch + HTTP API — 14/14 tests pass, all 4 tasks done
  - T01: [x] Core types, JobQueue, ServerState, unit tests
  - T02: [x] dispatch_loop, run_job_task, CancellationToken broadcast
  - T03: [x] DirectoryWatcher with atomic file-move
  - T04: [x] HTTP API (axum) with 4 routes + 6 integration tests
- S03: [ ] Ratatui TUI + Server Config + Graceful Shutdown — depends:[S01,S02]

## Blockers
- None

## Next Action
Plan and execute S03: wire all S02 components into `smelt serve` CLI subcommand with ServerConfig, Ratatui TUI, and Ctrl+C graceful shutdown.
