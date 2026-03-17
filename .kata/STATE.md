# Kata State

**Active Milestone:** M001 — Docker-First Infrastructure MVP
**Active Slice:** none — M001 COMPLETE
**Status:** All 6 slices done. 20/20 docker_lifecycle tests passing. Full milestone definition of done satisfied.
**Phase:** Complete

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
M001 complete. Ready to start M002 planning.
