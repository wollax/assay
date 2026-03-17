---
id: T01
parent: S05
milestone: M001
provides:
  - JobMonitor struct with phase tracking, container ID, sessions, timestamps, PID
  - RunState TOML persistence to .smelt/run-state.toml
  - compute_job_timeout() helper returning Duration from manifest
  - JobPhase enum with 9 lifecycle states
key_files:
  - crates/smelt-core/src/monitor.rs
  - crates/smelt-core/src/lib.rs
key_decisions:
  - Used u64 Unix timestamps instead of ISO 8601 strings to avoid adding chrono dependency
  - Used SmeltError::Io and SmeltError::Config structured variants for all error paths
patterns_established:
  - State file persistence pattern: create_dir_all + toml::to_string_pretty + fs::write
  - Cleanup tolerates NotFound (idempotent teardown)
observability_surfaces:
  - .smelt/run-state.toml — TOML file with job_name, phase, container_id, sessions, started_at, updated_at, pid
  - Phase values: provisioning, writing_manifest, executing, collecting, tearing_down, complete, failed, timeout, cancelled
duration: ~10min
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T01: Add JobMonitor and timeout helpers to smelt-core

**Added `JobMonitor` with TOML-serialized run state persistence and `compute_job_timeout()` helper to smelt-core.**

## What Happened

Created `crates/smelt-core/src/monitor.rs` with:
- `JobPhase` enum (9 variants, serde snake_case serialization)
- `RunState` struct with job_name, phase, container_id, sessions, timestamps, PID
- `JobMonitor` struct with `new()`, `set_phase()`, `set_container()`, `write()`, `read()`, `cleanup()` methods
- `compute_job_timeout()` function that extracts max session timeout from a manifest

Registered `pub mod monitor` in `lib.rs` with re-exports of all public types.

## Verification

- `cargo test -p smelt-core -- monitor::tests` — **11 tests passed** (new_monitor_initial_state, phase_transitions, set_container, write_and_read_roundtrip, cleanup_removes_file, cleanup_missing_file_ok, read_missing_file, compute_timeout_uses_max_session, compute_timeout_fallback, run_state_toml_serialization, set_phase_writes_to_disk)
- `cargo test --workspace` — **105 passed**, 0 failed, no regressions

### Slice-level verification status (T01 is intermediate):
- ✅ `cargo test -p smelt-core -- monitor::tests` — 11 pass
- ⏳ `cargo test -p smelt-core -- timeout::tests` — not yet created (future task)
- ⏳ `cargo test -p smelt-cli -- status` — not yet created (future task)
- ⏳ `cargo test -p smelt-cli --test docker_lifecycle -- timeout` — not yet created (future task)
- ⏳ `cargo test -p smelt-cli --test docker_lifecycle -- signal` — not yet created (future task)
- ✅ `cargo test --workspace` — 105 pass, zero regressions

## Diagnostics

Inspect running job state: `cat .smelt/run-state.toml` — shows TOML with phase, container_id, pid, timestamps. Terminal failure phases: `failed`, `timeout`, `cancelled`. Stale state detectable by comparing pid with running processes.

## Deviations

- Used u64 Unix timestamps instead of ISO 8601 strings — simpler, avoids new dependency, still machine-readable. The task plan suggested this as an acceptable alternative.
- Test manifests required matching the actual `JobManifest` schema (runtime field in environment, `target` not `target_branch` in merge, no `key_env` in credentials) — adjusted test TOML accordingly.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/monitor.rs` — New module with JobMonitor, JobPhase, RunState, compute_job_timeout, 11 unit tests
- `crates/smelt-core/src/lib.rs` — Added `pub mod monitor` and re-exports
