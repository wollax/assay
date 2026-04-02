---
id: T01
parent: S04
milestone: M003
provides:
  - JobMonitor::write/read/cleanup use state.toml (not run-state.toml)
  - JobMonitor::read_legacy(base_dir) reads legacy flat run-state.toml for backward compat
  - run.rs state_dir is .smelt/runs/<manifest.job.name>
  - watch.rs execute() uses .smelt/runs/<args.job_name> as state_dir; persist_run_state writes state.toml
  - status.rs optional positional job_name arg; execute() routes to read() or read_legacy() accordingly
key_files:
  - crates/smelt-core/src/monitor.rs
  - crates/smelt-cli/src/commands/run.rs
  - crates/smelt-cli/src/commands/watch.rs
  - crates/smelt-cli/src/commands/status.rs
key_decisions:
  - "state file renamed run-state.toml → state.toml; per-job path is .smelt/runs/<job-name>/state.toml (D034 superseded)"
  - "read_legacy() is a static method on JobMonitor to keep backward compat isolated at the API boundary"
  - "status.rs uses positional optional arg (not a flag) so `smelt status my-job` reads cleanly"
  - "watch execute() error message includes state_dir path for diagnosability when no PR was created"
patterns_established:
  - "per-job state isolation: .smelt/runs/<job-name>/state.toml is canonical state path from S04 onward"
  - "read_legacy() pattern: backward-compat reads always go through an explicitly named method, never through read()"
observability_surfaces:
  - "smelt status <job-name> reads .smelt/runs/<name>/state.toml"
  - "smelt status (no args) reads legacy .smelt/run-state.toml"
  - "smelt watch <job-name> error message includes expected state path for diagnosis"
duration: 20min
verification_result: passed
completed_at: 2026-03-21T00:00:00Z
blocker_discovered: false
---

# T01: Migrate JobMonitor state path from flat file to per-job directories

**Per-job state isolation: `JobMonitor` writes/reads `state.toml` under `.smelt/runs/<job-name>/`, with `read_legacy()` for backward-compat and optional positional `job_name` in `smelt status`.**

## What Happened

Renamed `run-state.toml` → `state.toml` in all three `JobMonitor` methods (`write`, `read`, `cleanup`). Added `JobMonitor::read_legacy(base_dir)` which reads the old flat `{base_dir}/run-state.toml` path — this is the only entry point for legacy state files.

Updated `run.rs` state_dir computation from `.smelt` to `.smelt/runs/<manifest.job.name>`. Because `JobMonitor::write()` already calls `fs::create_dir_all(&self.state_dir)`, the nested directory is created automatically on first write.

Fixed `watch.rs` `execute()` to use `.smelt/runs/<args.job_name>` as state_dir and updated `persist_run_state()` to write `state.toml`. The error message when no PR was created now includes the expected state path for diagnosability. Updated the `write_state_to_dir()` test helper to mirror the per-job layout.

Updated `status.rs` `StatusArgs` with an optional positional `job_name: Option<String>`. In `execute()`, when `job_name` is `Some(name)`, reads from `.smelt/runs/<name>/state.toml` via `JobMonitor::read()`; when `None`, calls `JobMonitor::read_legacy()` against `.smelt`. Updated all test helpers and added a `test_status_legacy_backward_compat` test.

Added three new `monitor.rs` unit tests: `test_read_legacy_reads_flat_file`, `test_state_path_resolution`, and `test_cleanup_uses_state_toml`.

## Verification

- `cargo test -p smelt-core`: 127 passed, 0 failed (includes 3 new monitor tests)
- `cargo test -p smelt-cli`: all passed (includes new `test_status_legacy_backward_compat`)
- `test_state_path_resolution` creates `.smelt/runs/my-job/`, calls `write()`, asserts `state.toml` exists and `run-state.toml` does not
- `test_read_legacy_reads_flat_file` manually writes `run-state.toml`, calls `read_legacy()`, asserts fields match
- `test_cleanup_uses_state_toml` verifies cleanup removes `state.toml`

## Diagnostics

- `smelt status <job-name>` → reads `.smelt/runs/<job-name>/state.toml`
- `smelt status` (no args) → reads legacy `.smelt/run-state.toml`
- `smelt watch <job-name>` with missing PR URL → error message includes `state_dir.display()` for path diagnosis

## Deviations

None — implementation followed the task plan exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-core/src/monitor.rs` — write/read/cleanup use state.toml; added read_legacy(); added 3 new tests; updated 2 existing tests
- `crates/smelt-cli/src/commands/run.rs` — state_dir now includes .smelt/runs/<job.name>
- `crates/smelt-cli/src/commands/watch.rs` — execute() uses per-job state_dir; persist_run_state writes state.toml; test helper updated
- `crates/smelt-cli/src/commands/status.rs` — optional positional job_name; execute() routes to read() or read_legacy(); test helpers and tests updated
