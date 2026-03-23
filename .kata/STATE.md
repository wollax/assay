# Kata State

**Active Milestone:** M006 — Parallel Dispatch Daemon
**Active Slice:** S03 — Ratatui TUI + Server Config + Graceful Shutdown
**Phase:** Complete — milestone M006 is complete; all 3 slices done

## Recent Decisions
- D107: Tracing subscriber branched in main() before dispatch — file appender for TUI mode, stderr for others
- D106: TUI shutdown coordination via Arc<AtomicBool> shared between tokio runtime and std::thread
- D105: HTTP POST persists TOML body via std::mem::forget(TempPath) — decouples file lifetime from handler scope
- D104: ServerState::complete() sets Retrying in-place (not re-enqueue) — single entry per job in VecDeque
- D103: JobId uses atomic u64 counter for deterministic test IDs
- D102: ServerConfig is a separate TOML file, not embedded in job manifests
- D101: axum for HTTP API, ratatui + crossterm for TUI
- D100: Queue pickup via file-move (queue_dir → queue_dir/dispatched/) — atomic, restart-safe

## Completed Milestones
- M001: ✅ Docker-First Infrastructure MVP (2026-03-17)
- M002: ✅ Real Assay Integration (2026-03-17)
- M003: ✅ GitHub Forge + PR Lifecycle (2026-03-21)
- M004: ✅ Docker Compose Runtime (2026-03-23)
- M005: ✅ Kubernetes Runtime (2026-03-23, pending live UAT)
- M006: ✅ Parallel Dispatch Daemon (2026-03-23, pending live UAT for TUI + Ctrl+C)

## M006 Slices
- S01: ✅ JobQueue + In-Process Dispatch — concurrent dispatch, CancellationToken, all tests pass
- S02: ✅ Directory Watch + HTTP API — DirectoryWatcher + axum API, 14/14 tests pass
- S03: ✅ Ratatui TUI + Server Config + Graceful Shutdown — final assembly, cargo test --workspace green
  - T01: [x] ServerConfig TOML struct + examples/server.toml
  - T02: [x] smelt serve CLI subcommand wiring (no TUI)
  - T03: [x] Ratatui TUI background thread
  - T04: [x] Wire TUI + tracing redirect; cargo test --workspace green

## Requirements Status
- R023, R024, R025 — all validated by M006 completion
- Active requirements: 0

## Blockers
- None

## Next Action
M006 is complete. Squash-merge S03 branch to main via kata extension. Manual UAT (live TUI + Ctrl+C with real Docker jobs) available in S03-UAT.md.
