# Kata State

**Active Milestone:** M008 — Remote worker dispatch via SSH
**Active Slice:** S04 — Dispatch routing + round-robin + TUI/API worker field
**Active Task:** None — slice not yet planned
**Phase:** Planning

## Recent Decisions
- scp_from uses -r flag unconditionally for directory copy (S03/T01)
- MockSshClient uses separate scp_from_results queue independent from scp_results (S03/T01)
- sync_state_back computes remote path as /tmp/.smelt/runs/<job_name>/ using job_name not JobId (S03/T02)

## Blockers
- None

## Next Action
Plan and execute S04: wire dispatch routing (local vs SSH), round-robin worker selection, offline-worker failover, worker_host field in API/TUI, end-to-end integration test.
