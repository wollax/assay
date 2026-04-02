# S01: M011 Leftover Cleanup — Tracing Migration & Flaky Test Fix

**Goal:** All `eprintln!` calls in smelt-cli (except two documented exceptions) are replaced with structured `tracing` events; `test_cli_run_invalid_manifest` uses a 30s timeout and no longer flakes. 298+ tests pass.
**Demo:** `cargo test --workspace` passes with 0 failures; `smelt run --dry-run` on a valid manifest outputs clean bare-message text to stderr (no timestamps/levels); `SMELT_LOG=debug smelt run --dry-run` outputs full-format diagnostic text with timestamps/levels/targets.

## Must-Haves

- All `eprintln!` in smelt-cli replaced with `tracing` macros except `main.rs:76` (top-level error handler, D139) and `serve/tui.rs` (TUI error after ratatui restore)
- Default env filter changed from `"warn"` to `"smelt_cli=info,smelt_core=info,warn"` so `info!` progress messages appear
- Default stderr subscriber uses `.without_time().with_target(false).with_level(false)` for bare-message output matching current `eprintln!` behavior
- When `SMELT_LOG` or `RUST_LOG` is explicitly set, full format (with time/target/level) is used for operator diagnostics
- TUI file appender path retains full format (levels are useful in log files)
- `test_cli_run_invalid_manifest` timeout increased from 10s to 30s
- Integration test assertions on stderr substrings (`"Writing manifest..."`, `"Executing assay run..."`, `"Assay complete"`, `"Container removed"`, `"Error"`) still pass
- `cargo test --workspace` passes (298+ tests, 0 failures)
- `cargo clippy --workspace` clean
- `cargo doc --workspace --no-deps` clean

## Proof Level

- This slice proves: contract (all eprintln! calls migrated; filter and format configured correctly; existing tests pass)
- Real runtime required: no (existing integration tests exercise the subscriber; no new Docker tests needed)
- Human/UAT required: no (automated test suite is sufficient; UAT covers manual visual inspection of output format)

## Verification

- `cargo test --workspace` — 298+ tests pass, 0 failures
- `cargo clippy --workspace -- -D warnings` — 0 warnings
- `cargo doc --workspace --no-deps` — 0 warnings
- `rg 'eprintln!' crates/smelt-cli/src/ --count-matches` — exactly 2 results: `main.rs:1` and `serve/tui.rs:1`
- `rg 'from_secs(10)' crates/smelt-cli/tests/docker_lifecycle.rs` — 0 results (timeout changed to 30s)

## Observability / Diagnostics

- Runtime signals: All smelt-cli output now flows through `tracing` — operators control verbosity via `SMELT_LOG` env var (D107). Default shows `info` for smelt crates, `warn` for dependencies.
- Inspection surfaces: `SMELT_LOG=debug` activates full-format output with timestamps, levels, and targets for any smelt command. TUI mode logs to `.smelt/serve.log`.
- Failure visibility: Error-level tracing events replace `eprintln!` error messages — same content, now filterable and redirectable.
- Redaction constraints: None — no secrets flow through tracing output.

## Integration Closure

- Upstream surfaces consumed: None (cleanup slice, no dependencies)
- New wiring introduced in this slice: Modified subscriber init in `main.rs` (format config + default filter change); all CLI output now routed through tracing
- What remains before the milestone is truly usable end-to-end: S02 (TrackerSource trait + config), S03 (GitHub backend), S04 (Linear backend), S05 (dispatch integration + state backend passthrough)

## Tasks

- [x] **T01: Configure tracing subscriber format and default filter** `est:30m`
  - Why: The subscriber init in `main.rs` must produce bare-message output (matching current `eprintln!` behavior) by default, and full diagnostic output when `SMELT_LOG` is set. The default filter must change from `"warn"` to `"smelt_cli=info,smelt_core=info,warn"` so `info!`-level progress messages appear after migration.
  - Files: `crates/smelt-cli/src/main.rs`
  - Do: Restructure the subscriber init block — detect whether `SMELT_LOG`/`RUST_LOG` is explicitly set; if not, use bare format (`.without_time().with_target(false).with_level(false)`) with target-scoped default filter `"smelt_cli=info,smelt_core=info,warn"`; if set, use full format with the user-provided filter. TUI file appender always uses full format.
  - Verify: `cargo test --workspace` passes; `cargo clippy --workspace -- -D warnings` clean
  - Done when: Subscriber init produces bare-message output by default and full-format output when SMELT_LOG is set; existing tests unchanged

- [x] **T02: Migrate all eprintln! calls to tracing macros** `est:45m`
  - Why: Core migration — 51 `eprintln!` calls across 6 files must become `info!`/`warn!`/`error!` calls with correct log levels, preserving exact message text for integration test compatibility.
  - Files: `crates/smelt-cli/src/commands/run/phases.rs`, `crates/smelt-cli/src/commands/watch.rs`, `crates/smelt-cli/src/commands/status.rs`, `crates/smelt-cli/src/commands/run/dry_run.rs`, `crates/smelt-cli/src/commands/init.rs`, `crates/smelt-cli/src/commands/list.rs`
  - Do: Replace each `eprintln!` with the appropriate tracing macro per the level mapping in S01-RESEARCH.md. Progress messages → `info!`, warnings → `warn!`, errors/failures → `error!`. Preserve exact message text (substrings like "Writing manifest...", "Assay complete", "Container removed" must remain). Leave `main.rs:76` and `serve/tui.rs:27` as `eprintln!`.
  - Verify: `rg 'eprintln!' crates/smelt-cli/src/ --count-matches` shows exactly 2 results (main.rs:1, serve/tui.rs:1); `cargo test --workspace` passes; `cargo clippy --workspace -- -D warnings` clean
  - Done when: All 51 `eprintln!` calls migrated; only 2 documented exceptions remain; all tests pass

- [x] **T03: Fix flaky test timeout and final verification** `est:15m`
  - Why: `test_cli_run_invalid_manifest` uses a 10s subprocess timeout that's too short during cold builds, causing intermittent failures (R061). Final sweep confirms all verification criteria.
  - Files: `crates/smelt-cli/tests/docker_lifecycle.rs`
  - Do: Change `Duration::from_secs(10)` to `Duration::from_secs(30)` on line 813. Run full verification suite: `cargo test --workspace`, `cargo clippy`, `cargo doc`, `rg` checks for remaining `eprintln!` and old timeout.
  - Verify: `rg 'from_secs(10)' crates/smelt-cli/tests/docker_lifecycle.rs` returns 0 results; `cargo test --workspace` passes (298+ tests); `cargo clippy --workspace -- -D warnings` clean; `cargo doc --workspace --no-deps` clean
  - Done when: Timeout changed to 30s; all 298+ tests pass; all verification commands green; R061 and R062 resolved

## Files Likely Touched

- `crates/smelt-cli/src/main.rs`
- `crates/smelt-cli/src/commands/run/phases.rs`
- `crates/smelt-cli/src/commands/watch.rs`
- `crates/smelt-cli/src/commands/status.rs`
- `crates/smelt-cli/src/commands/run/dry_run.rs`
- `crates/smelt-cli/src/commands/init.rs`
- `crates/smelt-cli/src/commands/list.rs`
- `crates/smelt-cli/tests/docker_lifecycle.rs`
