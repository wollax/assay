# S03: State sync back via scp — UAT

**Milestone:** M008
**Written:** 2026-03-24

## UAT Type

- UAT mode: artifact-driven
- Why this mode is sufficient: S03 adds a library function (`sync_state_back`) and a trait method (`scp_from`) consumed by S04's dispatch loop — there is no CLI surface or UI to exercise. Contract correctness is proven by mock unit tests; the gated integration test proves real scp behavior against localhost SSH.

## Preconditions

- SSH server running on localhost (sshd enabled)
- Current user can SSH to localhost without password prompt (pubkey auth)
- `SMELT_SSH_TEST=1` environment variable set
- `cargo` and Rust toolchain installed

## Smoke Test

```bash
cargo test -p smelt-cli --lib -- ssh::tests::test_sync_state_back_mock_success
```
Expected: 1 test passed, 0 failures.

## Test Cases

### 1. scp_from argument construction

1. Run: `cargo test -p smelt-cli --lib -- ssh::tests::test_scp_from_args_recursive`
2. **Expected:** Test passes — verifies `-r` flag present, remote `user@host:/path` before local path, `-P` for custom port.

### 2. sync_state_back mock success

1. Run: `cargo test -p smelt-cli --lib -- ssh::tests::test_sync_state_back_mock_success`
2. **Expected:** Test passes — local state directory created, scp_from called with correct paths.

### 3. sync_state_back mock failure

1. Run: `cargo test -p smelt-cli --lib -- ssh::tests::test_sync_state_back_mock_failure`
2. **Expected:** Test passes — scp_from error propagated as Err from sync_state_back.

### 4. Full state sync round-trip (gated)

1. Run: `SMELT_SSH_TEST=1 cargo test -p smelt-cli -- --include-ignored test_state_sync_round_trip`
2. **Expected:** Test passes — manifest delivered to localhost, remote state created via ssh exec, synced back via scp, local file verified as valid TOML with correct job_name and phase fields.

### 5. Full workspace green

1. Run: `cargo test --workspace`
2. **Expected:** All tests pass, 0 failures.

## Edge Cases

### scp failure does not crash the dispatcher

1. Run: `cargo test -p smelt-cli --lib -- ssh::tests::test_sync_state_back_mock_failure`
2. **Expected:** `sync_state_back()` returns `Err` (not panic); caller can log warning and continue.

## Failure Signals

- Any test in `ssh::tests::test_scp_from*` or `ssh::tests::test_sync_state_back*` fails
- Gated integration test `test_state_sync_round_trip` fails when `SMELT_SSH_TEST=1` is set
- `cargo test --workspace` reports failures in smelt-cli

## Requirements Proved By This UAT

- R027 (partial) — proves the state-sync-back leg: `sync_state_back()` pulls remote `.smelt/runs/<job_name>/` to the dispatcher's local filesystem. Contract proven by mock tests; real scp proven by gated localhost test.

## Not Proven By This UAT

- R027 full validation requires S04: dispatch routing, round-robin worker selection, offline-worker failover, `worker_host` field in API/TUI, end-to-end integration with 2 mock workers
- `smelt status <job>` reading synced state — requires S04 wiring to actually dispatch a job and write state via the normal path
- Behavior when remote state directory is empty or partial (scp succeeds but files are missing)

## Notes for Tester

- The gated test (`test_state_sync_round_trip`) requires localhost SSH — if your machine doesn't have sshd enabled, skip test case 4. The mock-based tests (cases 1-3) are sufficient for contract verification.
- The remote path `/tmp/.smelt/runs/<job_name>/` is cleaned up by the integration test on completion.
