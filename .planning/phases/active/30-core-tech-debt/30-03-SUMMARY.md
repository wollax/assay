# 30-03 Summary: Guard Daemon Persistence Hardening

## Result: PASS

**Duration:** ~5 minutes
**Commits:** 4 (2 task commits + 2 pre-existing fixes)

## Tasks Completed

### Task 1: PID file fsync (CORE-07)
- Replaced `fs::write()` in `create_pid_file()` with explicit `File::create` + `write_all` + `sync_all`
- Added `use std::io::Write` import
- All 8 PID tests pass unchanged
- **Commit:** `276630b fix(30-03): fsync PID file writes in create_pid_file`

### Task 2: Store project_dir in GuardDaemon (CORE-08)
- Added `project_dir: PathBuf` field to `GuardDaemon` struct
- Updated `GuardDaemon::new()` to accept `project_dir` parameter
- Updated `try_save_checkpoint()` to use `&self.project_dir` instead of `std::env::current_dir()`
- Updated `start_guard()` in `mod.rs` to accept and pass `project_dir`
- Updated CLI caller in `context.rs` to pass `&root` as `project_dir`
- Updated test helper `make_daemon()` and direct `GuardDaemon::new()` test call
- Added `project_dir` assertion in `guard_daemon_new_creates_valid_struct` test
- All 52 guard tests pass
- **Commit:** `e81ceb9 fix(30-03): store project_dir in GuardDaemon for checkpoint saves`

## Files Modified

- `crates/assay-core/src/guard/pid.rs` — fsync on PID file write
- `crates/assay-core/src/guard/daemon.rs` — project_dir field, updated constructor/checkpoint/tests
- `crates/assay-core/src/guard/mod.rs` — project_dir parameter on start_guard()
- `crates/assay-cli/src/commands/context.rs` — pass project root to start_guard()

## Verification

- `just ready` passes (fmt-check, lint, test, deny, plugin-version)
- No `std::env::current_dir()` calls remain in daemon.rs
- All success criteria met

## Pre-existing Issues Fixed

- Formatting issue in `crates/assay-core/src/history/mod.rs` (from 30-02 plan)
- Plugin version mismatch `plugins/claude-code/.claude-plugin/plugin.json` (0.1.0 -> 0.2.0)
