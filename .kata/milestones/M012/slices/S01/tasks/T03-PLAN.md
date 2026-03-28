---
estimated_steps: 3
estimated_files: 1
---

# T03: Fix flaky test timeout and final verification

**Slice:** S01 — M011 Leftover Cleanup — Tracing Migration & Flaky Test Fix
**Milestone:** M012

## Description

Increase the `test_cli_run_invalid_manifest` subprocess timeout from 10s to 30s (R061), then run the complete verification suite to confirm all slice must-haves are met. This is a single constant change plus final sweep.

The flaky test at `docker_lifecycle.rs:813` uses `assert_cmd::Command::timeout(Duration::from_secs(10))`. When cargo needs to link the binary (cold cache, incremental rebuild), 10s is insufficient and the test fails with a timeout error rather than the expected validation error. 30s provides ample headroom.

## Steps

1. Open `crates/smelt-cli/tests/docker_lifecycle.rs` line 813. Change `Duration::from_secs(10)` to `Duration::from_secs(30)`.
2. Run the full verification suite:
   - `cargo test --workspace` — 298+ tests, 0 failures
   - `cargo clippy --workspace -- -D warnings` — 0 warnings
   - `cargo doc --workspace --no-deps` — 0 warnings
   - `rg 'eprintln!' crates/smelt-cli/src/ --count-matches` — exactly 2 results (main.rs:1, serve/tui.rs:1)
   - `rg 'from_secs(10)' crates/smelt-cli/tests/docker_lifecycle.rs` — 0 results
3. Confirm all slice must-haves are met. If any verification step fails, diagnose and fix.

## Must-Haves

- [ ] `test_cli_run_invalid_manifest` timeout changed from 10s to 30s
- [ ] `rg 'from_secs(10)' crates/smelt-cli/tests/docker_lifecycle.rs` returns 0 results
- [ ] `cargo test --workspace` passes (298+ tests, 0 failures)
- [ ] `cargo clippy --workspace -- -D warnings` — 0 warnings
- [ ] `cargo doc --workspace --no-deps` — 0 warnings
- [ ] `rg 'eprintln!' crates/smelt-cli/src/ --count-matches` — exactly main.rs:1, serve/tui.rs:1

## Verification

- All 5 verification commands listed above pass
- R061 (flaky test) is resolved by the timeout increase
- R062 (full tracing migration) is resolved by T01+T02 work confirmed in this sweep

## Observability Impact

- Signals added/changed: None
- How a future agent inspects this: `rg 'from_secs' crates/smelt-cli/tests/docker_lifecycle.rs` shows the timeout value
- Failure state exposed: None

## Inputs

- `crates/smelt-cli/tests/docker_lifecycle.rs` — line 813, current 10s timeout
- T01 and T02 completed — subscriber configured, all eprintln! migrated

## Expected Output

- `crates/smelt-cli/tests/docker_lifecycle.rs` — timeout changed from 10s to 30s
- All verification commands green — slice is complete
