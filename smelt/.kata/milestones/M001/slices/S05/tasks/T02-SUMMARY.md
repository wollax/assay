---
id: T02
parent: S05
milestone: M001
provides:
  - "`smelt status` CLI subcommand that reads `.smelt/run-state.toml` and prints formatted job progress"
  - "StatusArgs struct with --dir flag for project root"
  - "PID-based stale detection with warning"
  - "Exit code 0 for active job, 1 for no job / stale / terminal phase"
key_files:
  - crates/smelt-cli/src/commands/status.rs
  - crates/smelt-cli/src/commands/mod.rs
  - crates/smelt-cli/src/main.rs
key_decisions:
  - "Terminal phases (complete, failed, timeout, cancelled) return exit code 1 like missing state — only active jobs return 0"
patterns_established:
  - "CLI subcommand pattern: StatusArgs (clap Args) + async execute() -> Result<i32>"
  - "PID liveness check via `kill -0` on Unix, skip on non-Unix"
observability_surfaces:
  - "`smelt status` — user-facing inspection of running job state"
  - "Stale PID warning on stderr when process has died without cleanup"
duration: 8m
verification_result: passed
completed_at: 2026-03-17
blocker_discovered: false
---

# T02: Add `smelt status` CLI subcommand

**Added `smelt status` subcommand that reads `.smelt/run-state.toml` and prints formatted job progress with PID-based stale detection.**

## What Happened

Created `status.rs` with `StatusArgs` (optional `--dir` flag) and `execute()` function. The command reads the run state TOML file via `JobMonitor::read()`, checks PID liveness using `kill -0`, and prints formatted output showing job name, phase, container ID, sessions, PID, and elapsed time. Returns exit code 0 for active jobs, 1 for missing state / stale PID / terminal phases. Added `Status` variant to the `Commands` enum in `main.rs` and registered the module in `mod.rs`.

## Verification

- `cargo test -p smelt-cli -- status` — 7 tests pass (no_state_file, active_job, stale_pid, format_elapsed × 3, is_terminal_phase)
- `cargo run -- status` with no state file → prints "No running job." to stderr, exits 1
- `cargo test --workspace` — 105 unit + 2 doc tests pass, zero regressions

### Slice-level verification status (intermediate task):
- ✅ `cargo test -p smelt-core -- monitor::tests` — passes (T01)
- ✅ `cargo test -p smelt-core -- timeout::tests` — passes (T01, covered by monitor::tests)
- ✅ `cargo test -p smelt-cli -- status` — passes (this task)
- ⏳ `cargo test -p smelt-cli --test docker_lifecycle -- timeout` — T03
- ⏳ `cargo test -p smelt-cli --test docker_lifecycle -- signal` — T03
- ✅ `cargo test --workspace` — all pass

## Diagnostics

- Run `smelt status` or `smelt status --dir /path/to/project` to inspect a running job
- Stale PID warning appears on stderr when the runner process has died without cleanup
- Exit code 0 = active job, 1 = no job / stale / terminal phase

## Deviations

- Added `toml` as a dev-dependency to smelt-cli for test helpers (writing state files directly in tests)
- Added extra helper tests for `format_elapsed()` and `is_terminal_phase()` beyond the 3 required

## Known Issues

None

## Files Created/Modified

- `crates/smelt-cli/src/commands/status.rs` — new module with StatusArgs, execute(), PID detection, formatting, 7 tests
- `crates/smelt-cli/src/commands/mod.rs` — added `pub mod status`
- `crates/smelt-cli/src/main.rs` — added `Status` variant to Commands enum and dispatch arm
- `crates/smelt-cli/Cargo.toml` — added `toml` dev-dependency for tests
