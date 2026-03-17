# Kata State

**Active Milestone:** M002 — Real Assay Integration
**Active Slice:** none — context written, planning next
**Status:** M002-CONTEXT.md written. Stale test fixed (clean baseline). Ready for M002 roadmap planning.
**Phase:** Planning

## Recent Decisions
- D034: Monitor state file as TOML at `.smelt/run-state.toml`, single-job model
- D035: Timeout from max session timeout, fallback to config default
- D036: Signal handling via `tokio::select!` with `ctrl_c()` + `sleep(timeout)` racing exec
- D037: Testable cancellation via generic future parameter (not CancellationToken), avoids tokio-util dep
- D038: DockerProvider::teardown() tolerates 404 on remove_container for idempotent double-teardown (S05)
- D039: E2E tests chain phases manually — run_with_cancellation() can't inject mock assay setup (S06)
- D040: Mock assay binary placed at /usr/local/bin/assay to match AssayInvoker::build_run_command() PATH lookup (S06)
- D041: Pre-clean orphan smelt containers at test start for tests asserting container absence (S06)
- D042: Orphan-check scoped to job-specific label value (label=smelt.job=<name>) to avoid false positives under concurrent test execution (S06)

## Progress
- S01: ✅ Scaffold, Manifest & Dry-Run CLI (4 tasks)
- S02: ✅ Docker Container Provisioning & Teardown (4 tasks)
- S03: ✅ Repo Mount & Assay Execution (3 tasks)
- S04: ✅ Result Collection & Branch Output (2 tasks)
- S05: ✅ Job Monitoring, Timeout & Graceful Shutdown (3 tasks)
- S06: ✅ End-to-End Integration (3 tasks, 20 docker_lifecycle tests passing)

## Blockers
- None

## Known Issues
- `run_without_dry_run_attempts_docker` in `dry_run.rs` is a pre-existing failing test (test logic incorrect — asserts Docker unavailable but Docker is present). Not introduced by any M001 slice. Should be fixed before starting M002.

## Next Action
Write M002 roadmap. Read M002-CONTEXT.md (especially the Assay contract gap section), then plan slices starting with: S01 fixing AssayInvoker contract + assay init in container, S02 real binary integration test, S03 result collection compatibility with Assay's own merge output. The bridging strategy (generate spec files vs. reference existing specs) must be decided during planning.
