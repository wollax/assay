# S01: WorkerConfig + SSH connection proof — UAT

**Milestone:** M008
**Written:** 2026-03-23

## UAT Type

- UAT mode: artifact-driven (with optional live-runtime gated tests)
- Why this mode is sufficient: All config contract verification is automated via `cargo test`. The SSH subprocess proof is automated via gated integration tests (`SMELT_SSH_TEST=1`). No human-observable UI surface was introduced in this slice — all outputs are Rust test results and log lines.

## Preconditions

For artifact-driven checks (always runnable):
- Rust toolchain installed; `cargo test --workspace` works

For gated SSH tests (optional, requires live runtime):
- `sshd` running on localhost with key-based auth configured for current user
- No service listening on `127.0.0.1:19222` (for the offline test)

## Smoke Test

```sh
cargo test --workspace
```

Expected: `155 passed; 0 failed; 2 ignored` (or similar count — the 2 ignored are the gated SSH tests).

## Test Cases

### 1. WorkerConfig roundtrip and defaults

```sh
cargo test -p smelt-cli test_worker_config
```

1. Run the command above
2. **Expected:** All `test_worker_config_*` tests pass — roundtrip, port default (22), deny_unknown_fields rejection, empty-host failure, empty-user failure

### 2. Existing server.toml files without [[workers]] still parse

```sh
cargo test -p smelt-cli test_server_config
```

1. Run the command above
2. **Expected:** All `test_server_config_*` tests pass — including `test_server_config_no_workers_parses`

### 3. SSH arg assembly (no real SSH call)

```sh
cargo test -p smelt-cli test_ssh_args
```

1. Run the command above
2. **Expected:** `test_ssh_args_build` and `test_ssh_args_build_custom_port` pass; args include `BatchMode=yes`, `StrictHostKeyChecking=accept-new`, `ConnectTimeout=3`, and `-p 2222` for non-default port

### 4. examples/server.toml documents [[workers]]

```sh
grep -A 6 '\[\[workers\]\]' examples/server.toml
```

1. Run the command above
2. **Expected:** Output shows a commented `[[workers]]` block with `host`, `user`, `key_env`, and `port` fields

### 5. (Gated) Live SSH exec to localhost

```sh
SMELT_SSH_TEST=1 cargo test -p smelt-cli -- --include-ignored test_ssh_exec_localhost
```

1. Ensure `sshd` is running on localhost with key-based auth
2. Run the command above
3. **Expected:** Test passes — `echo hello` returns `stdout == "hello"`, `exit_code == 0`

### 6. (Gated) Offline worker fast-fail

```sh
SMELT_SSH_TEST=1 cargo test -p smelt-cli -- --include-ignored test_ssh_probe_offline
```

1. Confirm nothing is listening on `127.0.0.1:19222`
2. Run the command above
3. **Expected:** Test passes — `probe()` returns `Err` within ≤ 4 seconds (bounded by `ConnectTimeout=3` + process overhead)

## Edge Cases

### Unknown field in [[workers]] entry

1. Add `unknown_field = "value"` to a `[[workers]]` entry in `server.toml`
2. Run `smelt serve --config server.toml` (or write a test TOML with the field and parse it)
3. **Expected:** Parse error — `WorkerConfig` uses `deny_unknown_fields`; clear TOML error referencing the unknown key

### Empty host in [[workers]]

1. Add `[[workers]]` with `host = ""` to `server.toml`
2. Attempt to parse and validate
3. **Expected:** `ServerConfig::validate()` returns error: `"invalid worker configuration:\n  worker[0]: host must not be empty"`

### key_env not set in environment

1. Configure a worker with `key_env = "NONEXISTENT_VAR"`
2. Call `SubprocessSshClient::exec()` with that worker
3. **Expected:** `WARN` log line noting the env var is missing; SSH falls back to default key (`~/.ssh/id_rsa` or agent); no hard error at exec time

## Failure Signals

- `cargo test --workspace` reports failures → S01 regression; check which test_worker_config or test_ssh test failed
- `grep '\[\[workers\]\]' examples/server.toml` returns no output → example block missing from server.toml
- Gated test `test_ssh_probe_offline` takes > 4 seconds → ConnectTimeout flag not being passed correctly to SSH subprocess
- `test_ssh_args_build` fails → `build_ssh_args()` arg order or flag values have changed; check for regressions in ssh.rs

## Requirements Proved By This UAT

- R027 (SSH worker pools / remote dispatch) — partially: this UAT proves the config schema (`WorkerConfig`, `ServerConfig::workers`) is correct and the SSH subprocess execution primitive (`SshClient`/`SubprocessSshClient`) works. The gated tests prove end-to-end subprocess invocation on a real localhost SSH connection.

## Not Proven By This UAT

- R027 full validation — end-to-end dispatch routing, manifest delivery via scp, remote `smelt run` execution, and state sync back are not covered here. These are S02, S03, and S04 responsibilities.
- Worker failover / re-queue behavior — S04 proves round-robin and offline-worker handling.
- `worker_host` field in `GET /api/v1/jobs` and TUI — S04.
- Real multi-machine SSH (non-localhost) — deferred to S04-UAT.md (human verification with a real remote host).

## Notes for Tester

- The 2 gated tests (`test_ssh_exec_localhost`, `test_ssh_probe_offline`) are marked `#[ignore]` and only run with `--include-ignored`. They pass automatically in standard CI. Run them manually to validate the SSH subprocess approach on a machine with `sshd`.
- If `sshd` is not running on localhost, the gated exec test will fail immediately (refused connection) rather than hanging — this is the expected fast-fail behavior.
- The offline test (`test_ssh_probe_offline`) uses port `19222` — ensure no service is bound there before running.
