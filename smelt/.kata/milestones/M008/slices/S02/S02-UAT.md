# S02: Manifest delivery + remote smelt run execution — UAT

**Milestone:** M008
**Written:** 2026-03-24

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: The core contract (scp a file to a remote host, execute `smelt run` via ssh, capture exit code) requires a real SSH session to prove. Mock tests validate the composition logic; this UAT proves the real transport works end-to-end.

## Preconditions

- SSH access to a test host (can be localhost with `sshd` running)
- `SMELT_SSH_TEST=1` environment variable set
- `SMELT_TEST_SSH_HOST` set to the target host (default: `localhost`)
- `SMELT_TEST_SSH_USER` set to the SSH user
- `SMELT_TEST_SSH_KEY` set to the path of the SSH private key
- `scp` and `ssh` binaries available on PATH
- A valid manifest TOML file available locally (e.g. `examples/job-manifest.toml`)

## Smoke Test

```bash
SMELT_SSH_TEST=1 cargo test -p smelt-cli -- --include-ignored test_manifest_delivery
```

This runs the gated integration test that scps a manifest to the test host and executes a command via SSH. Passes = smoke test passes.

## Test Cases

### 1. Manifest delivery via scp

1. Set SSH environment variables for a reachable host
2. Run `SMELT_SSH_TEST=1 cargo test -p smelt-cli -- --include-ignored test_manifest_delivery_and_remote_exec`
3. **Expected:** Test passes — manifest is delivered to `/tmp/smelt-<job_id>.toml` on the remote host

### 2. Manual scp verification

1. Create a small test manifest: `echo '[environment]\nimage = "alpine"' > /tmp/test-manifest.toml`
2. Manually scp it: `scp -o BatchMode=yes /tmp/test-manifest.toml <user>@<host>:/tmp/smelt-manual-test.toml`
3. Verify: `ssh <user>@<host> cat /tmp/smelt-manual-test.toml`
4. **Expected:** File contents match the original

### 3. Remote command execution exit code

1. `ssh <user>@<host> "exit 0"` — should return 0
2. `ssh <user>@<host> "exit 2"` — should return 2
3. `ssh <user>@<host> "nonexistent-command-xyz"` — should return 127
4. **Expected:** Exit codes match; exit code 127 corresponds to "command not found"

## Edge Cases

### Unreachable host

1. Attempt scp to a non-existent host: `scp -o ConnectTimeout=3 /tmp/test.toml badhost:/tmp/test.toml`
2. **Expected:** Fails within ~3 seconds with connection error

### Missing scp binary

1. Temporarily rename scp: not recommended in practice, but `which::which("scp")` should return an error if scp is not found
2. **Expected:** `deliver_manifest` returns an error before attempting the subprocess

### Large manifest file

1. Create a manifest > 1MB and attempt delivery
2. **Expected:** scp handles it without issue (no manifest size limit in the implementation)

## Failure Signals

- `cargo test --workspace` shows any new failures after S02 changes
- scp exits with non-zero but ssh.rs error message doesn't include stderr — indicates logging gap
- Exit code 127 from `run_remote_job` without a WARN log — indicates the 127-specific warning was not triggered
- `#[allow(dead_code)]` reappearing on `key_env` or `port` in config.rs — indicates dead code regression

## Requirements Proved By This UAT

- R027 (partial) — This UAT proves manifest delivery via scp and remote command execution via ssh. These are two of the four components required for R027 validation (the other two: state sync back [S03] and dispatch routing [S04]).

## Not Proven By This UAT

- State sync back from worker to dispatcher (S03 scope)
- Dispatch routing / round-robin worker selection (S04 scope)
- Full end-to-end: job submitted → dispatched to SSH worker → state synced back → visible in `smelt status` (S04 scope)
- Multi-worker failover (S04 scope)
- `worker_host` field in API/TUI (S04 scope)

## Notes for Tester

- The gated integration test (`SMELT_SSH_TEST=1`) is the primary automated proof. If it passes, the transport layer is working.
- On macOS, ensure `Remote Login` is enabled in System Settings > General > Sharing for localhost SSH testing.
- The test uses `--dry-run` semantics via direct `client.exec()` rather than `run_remote_job()` because `run_remote_job` hardcodes `smelt run <path>` without a dry-run flag.
- Mock-based unit tests (no SSH required) are the primary automated verification and run in `cargo test --workspace` without any special env vars.
