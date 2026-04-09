---
phase: 60-process-safety
plan: 02
subsystem: assay-core/pipeline
tags: [process-safety, killpg, sigterm, sigkill, relay-thread, panic-logging]
deps: []
tech_stack: [rust, libc]
key_files:
  - crates/assay-core/src/pipeline_checkpoint.rs
  - crates/assay-core/src/pipeline.rs
requirements: [SAFE-01, SAFE-04]
duration_secs: 127
commits:
  - hash: 4e15cef
    message: "feat(60-02): switch kill_agent_subprocess to killpg for process group termination"
  - hash: 7c5916b
    message: "feat(60-02): standardize inline abort to killpg and log relay thread panics"
---

Switch both process-kill paths from single-process `libc::kill` to `libc::killpg` and log relay thread panics with extracted payload instead of silently returning -1.

## Tasks

| # | Name | Status | Commit |
|---|------|--------|--------|
| 1 | Switch kill_agent_subprocess to killpg + update test | done | 4e15cef |
| 2 | Standardize inline abort to killpg + log relay panics | done | 7c5916b |

## Changes

### pipeline_checkpoint.rs
- `kill_agent_subprocess`: both SIGTERM and SIGKILL paths now call `libc::killpg(pid as libc::pid_t, sig)` instead of `libc::kill(pid as i32, sig)`
- SAFETY comments updated to describe killpg semantics (pgid == pid, entire process group)
- Test `kill_helper_terminates_long_running_process`: child spawns via `pre_exec(|| { libc::setpgid(0, 0); Ok(()) })` to isolate it in its own process group, preventing killpg from signaling the test runner

### pipeline.rs
- Checkpoint-abort inline kill: `libc::kill(-(pid as i32), SIGTERM)` → `libc::killpg(pid as libc::pid_t, SIGTERM)` (SAFETY comment updated)
- Relay join: `unwrap_or(-1)` → `match` arm that extracts panic payload via `downcast_ref::<&str>` / `downcast_ref::<String>` and logs via `tracing::error!(panic = %msg, "relay thread panicked")`

## Verification

- `rtk cargo test -p assay-core kill_helper` — 2 passed
- `rtk cargo test -p assay-core pipeline` — 34 passed
- `rtk cargo clippy -p assay-core -- -D warnings` — no issues
- No remaining `libc::kill(pid` in kill paths (only signal-0 liveness probes in tests)
- `libc::killpg` confirmed in `pipeline_checkpoint.rs` (×2) and `pipeline.rs` (×1)

## Deviations

None. Plan executed as specified.
