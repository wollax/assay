# S04: SshSyncBackend and CLI/MCP factory wiring — UAT

**Milestone:** M011
**Written:** 2026-03-27

## UAT Type

- UAT mode: live-runtime
- Why this mode is sufficient: Contract tests with mock scp/ssh binaries prove all 7 method implementations and injection safety at the unit level. The live-runtime UAT proves the actual SSH transport works end-to-end across a real network boundary — there is no substitute for real scp against a real remote host.

## Preconditions

1. A remote host reachable via SSH with key-based authentication (no password)
2. `scp` and `ssh` installed on the local machine
3. `assay-backends` compiled with `--features ssh` (or a binary built with ssh feature enabled)
4. Remote host has write access at some `remote_assay_dir` (e.g. `/tmp/assay-test/`)
5. A minimal `RunManifest` TOML with `state_backend = { type = "ssh", host = "<host>", remote_assay_dir = "/tmp/assay-test", user = "<user>" }`

## Smoke Test

Run a single-session `assay run` against the manifest with ssh backend configured and verify that `~/.assay/orchestrator/<run_id>/state.json` appears on the remote host:

```
ssh <user>@<host> ls /tmp/assay-test/orchestrator/
```

Expected: a directory named with the run_id containing `state.json`.

## Test Cases

### 1. push_session_event creates remote state file on first call

1. Configure manifest with `state_backend = { type = "ssh", host = "<host>", remote_assay_dir = "/tmp/assay-test" }`
2. Run `assay run <manifest>`
3. SSH to remote: `ssh <host> cat /tmp/assay-test/orchestrator/<run_id>/state.json`
4. **Expected:** Valid JSON `OrchestratorStatus` with session transitions

### 2. read_run_state deserializes remote state correctly

1. After a completed run (state.json exists on remote)
2. Run `assay orchestrate-status <run_id>` (or MCP `orchestrate_status` tool)
3. **Expected:** Status fields (phase, sessions, etc.) match what was pushed

### 3. Port override works

1. Configure manifest with non-standard SSH port: `state_backend = { type = "ssh", host = "<host>", port = 2222, remote_assay_dir = "/tmp/assay-test" }`
2. Run `assay run <manifest>`
3. **Expected:** `scp -P 2222` and `ssh -p 2222` used — visible in `RUST_LOG=debug` output

### 4. Path with spaces in remote_assay_dir does not split

1. Configure `remote_assay_dir = "/tmp/assay test dir"` (space in path)
2. Run `assay run <manifest>`
3. SSH to remote: `ssh <host> ls "/tmp/assay test dir/"`
4. **Expected:** State file created at the correct path; no "command not found" or split-path errors

### 5. send_message and poll_inbox round-trip

1. Run a two-session Mesh manifest with ssh backend
2. Let session A write a message to session B
3. **Expected:** Message file appears in remote inbox; poll returns content; remote file removed after polling

### 6. annotate_run pushes manifest path to remote

1. Run `assay run <manifest>` with ssh backend
2. SSH to remote: `ls /tmp/assay-test/orchestrator/<run_id>/`
3. **Expected:** `annotation.txt` exists containing the manifest path

## Edge Cases

### Remote host unreachable

1. Configure manifest with an invalid host
2. Run `assay run <manifest>`
3. **Expected:** `AssayError::io("scp push failed: ...")` with captured stderr; run fails with clear error, does not hang

### Remote dir not writable

1. Configure `remote_assay_dir = "/root/no-write"` without appropriate permissions
2. Run `assay run <manifest>`
3. **Expected:** Non-zero scp exit → `AssayError::io` with permission denied message; run fails immediately

### read_run_state on fresh run (no state yet)

1. Configure manifest with a new `run_id` that has no state on remote
2. Call `orchestrate_status` before any push
3. **Expected:** Returns gracefully (no panic, no crash) — `Ok(None)` propagated as "not found"

## Failure Signals

- `AssayError::io("scp push failed: ...")` with stderr containing "No such file or directory" — remote dir doesn't exist (ssh mkdir -p failed first)
- `AssayError::io("ssh mkdir failed: ...")` — SSH authentication failure or host unreachable
- State file at unexpected path (path components split) — injection not properly guarded (should not happen; signals mock fix didn't translate)
- `tracing::warn!(backend = "ssh")` in RUST_LOG output — ssh feature not compiled in, falling back to NoopBackend silently
- `poll_inbox` `tracing::warn!` on rm failure — message may re-deliver on next poll

## Requirements Proved By This UAT

- R078 — Real multi-machine scp push/pull proves `SshSyncBackend` works end-to-end: all 7 StateBackend methods via actual scp/ssh transport; CapabilitySet::all() enables all orchestrator features (messaging, gossip_manifest, annotations, checkpoints) against the remote host
- R079 — manifest `state_backend = { type = "ssh", ... }` routes to SshSyncBackend at runtime via `backend_from_config()` without any code change at CLI/MCP callsites

## Not Proven By This UAT

- Performance under high-frequency push_session_event (scp round-trip latency at mesh tick rates)
- Concurrent scp operations from multiple sessions to the same remote host (no locking implemented)
- Real LinearBackend and GitHubBackend end-to-end — separate UAT sessions required
- save_checkpoint_summary remote persistence wiring in orchestrator (M012 work)
- `poll_inbox` double-delivery on ssh rm failure (non-fatal warn path — observed only under adversarial conditions)
- WAN latency / high-latency SSH connections

## Notes for Tester

- Set `RUST_LOG=debug` to see scp/ssh command args logged before each spawn — confirms correct port flag case (uppercase `-P` for scp, lowercase `-p` for ssh)
- The injection safety test (path with spaces) is unit-tested at the contract level; UAT confirms the real scp binary receives the path as a single argument
- `cargo test -p assay-backends --features ssh -- --nocapture` shows mock scp arg logs for the unit contract tests
- Real multi-machine validation is the primary gap that unit tests cannot fill — focus UAT effort on the remote host being genuinely remote (not localhost)
