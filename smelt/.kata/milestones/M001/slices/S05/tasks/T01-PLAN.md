---
estimated_steps: 5
estimated_files: 3
---

# T01: Add JobMonitor and timeout helpers to smelt-core

**Slice:** S05 — Job Monitoring, Timeout & Graceful Shutdown
**Milestone:** M001

## Description

Create the `monitor.rs` module in smelt-core with `JobMonitor` — a struct that tracks job execution state (phase, container ID, session names, start time, PID) and persists it to `.smelt/run-state.toml` using serde/TOML serialization. Also add a `compute_job_timeout()` helper that extracts the effective job timeout from a manifest (max session timeout, falling back to `SmeltConfig::default_timeout`). Comprehensive unit tests cover state transitions, serialization round-trips, file I/O, and timeout computation.

## Steps

1. Create `crates/smelt-core/src/monitor.rs` with:
   - `JobPhase` enum: `Provisioning`, `WritingManifest`, `Executing`, `Collecting`, `TearingDown`, `Complete`, `Failed`, `Timeout`, `Cancelled` — derive Serialize/Deserialize with lowercase rename
   - `RunState` struct (serde): `job_name`, `phase`, `container_id` (Option), `sessions` (Vec<String>), `started_at` (ISO 8601 string), `updated_at`, `pid` (u32)
   - `JobMonitor` struct holding `state: RunState` and `state_dir: PathBuf`
   - `JobMonitor::new(job_name, sessions, state_dir)` — initializes state with `Provisioning` phase, records PID via `std::process::id()`, timestamps via `std::time::SystemTime`
   - `JobMonitor::set_phase(&mut self, phase)` — updates phase + `updated_at` timestamp, writes to disk
   - `JobMonitor::set_container(&mut self, container_id)` — sets container short ID
   - `JobMonitor::write(&self)` — serializes `RunState` to TOML, writes to `{state_dir}/run-state.toml`
   - `JobMonitor::read(state_dir) -> Result<RunState>` — reads and deserializes state file
   - `JobMonitor::cleanup(&self)` — removes state file (ignores NotFound)
   - `compute_job_timeout(manifest: &JobManifest, config_default: u64) -> Duration` — returns max session timeout as Duration, falling back to config_default if no sessions

2. Register `pub mod monitor` in `crates/smelt-core/src/lib.rs` with re-exports: `JobMonitor`, `JobPhase`, `RunState`, `compute_job_timeout`

3. Add `chrono` or use `std::time::SystemTime` + manual ISO formatting for timestamps. Prefer `SystemTime` to avoid a new dependency — format with `humantime` or a small helper. Actually, check if `humantime` is already a dep. If not, use a simple ISO 8601 formatter with `SystemTime::now().duration_since(UNIX_EPOCH)` and manual formatting, or just store Unix timestamps as u64 (simpler, still readable).

4. Write unit tests in `monitor.rs` `#[cfg(test)] mod tests`:
   - `test_new_monitor_initial_state` — verify initial phase is Provisioning, PID matches, sessions stored
   - `test_phase_transitions` — cycle through phases, verify `updated_at` changes
   - `test_set_container` — set container ID, verify it appears in state
   - `test_write_and_read_roundtrip` — write to tempdir, read back, verify all fields match
   - `test_cleanup_removes_file` — write then cleanup, verify file gone
   - `test_cleanup_missing_file_ok` — cleanup when no file exists doesn't error
   - `test_read_missing_file` — read from nonexistent path returns appropriate error
   - `test_compute_timeout_uses_max_session` — manifest with sessions [60, 300, 120] returns Duration::from_secs(300)
   - `test_compute_timeout_fallback` — empty sessions uses config default
   - `test_run_state_toml_serialization` — verify TOML output contains expected keys

5. Verify: `cargo test -p smelt-core -- monitor::tests` all pass, `cargo test --workspace` no regressions

## Must-Haves

- [ ] `JobMonitor` struct with phase tracking, container ID, sessions, timestamps, PID
- [ ] State file persistence to `.smelt/run-state.toml` via TOML serialization
- [ ] `compute_job_timeout()` helper returning `Duration` from manifest
- [ ] Cleanup method that removes state file (tolerates missing)
- [ ] 10+ unit tests covering round-trips, transitions, edge cases

## Verification

- `cargo test -p smelt-core -- monitor::tests` — all tests pass
- `cargo test --workspace` — no regressions (121+ tests still green)

## Observability Impact

- Signals added/changed: `JobMonitor` writes structured TOML state to disk on every phase transition — this is the primary observability surface for S05
- How a future agent inspects this: `cat .smelt/run-state.toml` shows current phase, container ID, elapsed time, PID
- Failure state exposed: Phase values `failed`, `timeout`, `cancelled` with `updated_at` timestamp indicate terminal failure states

## Inputs

- `crates/smelt-core/src/manifest.rs` — `JobManifest` and `SessionDef` types (timeout field)
- `crates/smelt-core/src/config.rs` — `SmeltConfig::default_timeout` for fallback
- S05-RESEARCH.md — recommended file-based approach with TOML, phase transitions, PID recording

## Expected Output

- `crates/smelt-core/src/monitor.rs` — new module with `JobMonitor`, `JobPhase`, `RunState`, `compute_job_timeout()`, 10+ unit tests
- `crates/smelt-core/src/lib.rs` — updated with `pub mod monitor` and re-exports
