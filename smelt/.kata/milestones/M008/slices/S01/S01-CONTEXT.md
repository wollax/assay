---
id: S01
milestone: M008
status: ready
---

# S01: WorkerConfig + SSH connection proof — Context

## Goal

Add `[[workers]]` config parsing to `ServerConfig` and prove that an SSH connection can be established, a command executed, and an offline host detected within 3s — via both a mockable `SshClient` trait and a localhost integration test.

## Why this Slice

S01 is the highest-risk slice of M008 — it validates the SSH execution primitive that everything else depends on. S02 (manifest delivery), S03 (state sync), and S04 (routing) cannot proceed until we know subprocess SSH works correctly, fast-fail is real (not just assumed), and the testability boundary (trait vs concrete) is settled. Doing this first collapses the biggest unknown before any dispatch logic is built.

## Scope

### In Scope

- `WorkerConfig { host: String, user: String, key_env: String, port: u16 }` with `#[derive(Deserialize)]`; `port` defaults to 22
- `ServerConfig::workers: Vec<WorkerConfig>` — absent `[[workers]]` section parses as empty vec; `deny_unknown_fields` on `WorkerConfig` (consistent with D017)
- `ssh_timeout_secs: u64` in `ServerConfig` (global, not per-worker) — defaults to 3; passed to all SSH subprocess invocations
- `SshClient` trait: `connect(&WorkerConfig) -> Result<SshSession>`, `exec(&str) -> Result<(String, i32)>` — mockable interface for S02/S03/S04 tests
- `SubprocessSshClient` as the concrete production implementation — shells out to `ssh`/`scp` consistent with D111 (subprocess pattern, D002 philosophy)
- `SubprocessSshClient::connect()` probe: attempts SSH with `ConnectTimeout=<ssh_timeout_secs>`, `BatchMode=yes`, `StrictHostKeyChecking=accept-new` — fast-fail on connection refused/timeout
- Localhost integration test (`SMELT_SSH_TEST=1` env var gate, gracefully skipped otherwise): connects to 127.0.0.1:22, execs `echo hello`, asserts stdout contains `hello` and exit code is 0
- Offline-worker test: connection to a port that refuses (e.g. 127.0.0.1:19999) returns error within `ssh_timeout_secs`
- `examples/server.toml` updated with a commented `[[workers]]` block documenting all fields
- Startup behaviour: warn and continue if any configured worker is unreachable at startup — probed again at dispatch time; no TUI changes in S01

### Out of Scope

- Manifest delivery, remote `smelt run` invocation — S02
- State sync back — S03
- Dispatch routing, round-robin, worker_host in TUI/API — S04
- `smelt_bin` per-worker config field — not needed; bare `smelt` on worker PATH is the contract (user's responsibility)
- `worker_host` written to persistent queue state — not persisted in M008; in-memory only until needed
- Per-worker timeout configuration — global `ssh_timeout_secs` only
- TUI changes to show worker status at startup
- Password auth, OIDC, ControlMaster/multiplexed SSH sessions
- Windows remote workers

## Constraints

- D111: SSH dispatch uses subprocess `ssh`/`scp`, not `openssh` or `ssh2` crates — no new native library dependencies
- D112: `key_env` field holds the name of the env var that contains the SSH private key *path*; the key value itself never appears in config
- D017: `deny_unknown_fields` on `WorkerConfig` — strict parsing, consistent with all other config structs
- SSH connection timeout must be enforced via `-o ConnectTimeout=<N>` and `-o BatchMode=yes` flags — prevents dispatch loop blocking
- `SMELT_SSH_TEST=1` env var gates localhost integration tests — consistent with `SMELT_K8S_TEST=1` pattern (D089); `cargo test --workspace` must stay green without it

## Integration Points

### Consumes

- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig` struct; `[[workers]]` added as `workers: Vec<WorkerConfig>` field with `#[serde(default)]`
- `examples/server.toml` — canonical config example; gains documented `[[workers]]` block

### Produces

- `WorkerConfig` struct — consumed by S02 (`deliver_manifest`), S03 (`sync_state_back`), S04 (dispatch routing)
- `ServerConfig::workers: Vec<WorkerConfig>` and `ServerConfig::ssh_timeout_secs: u64` — consumed by S04 dispatch logic
- `SshClient` trait + `SshSession` — mockable interface; `MockSshClient` used in S02/S03/S04 unit tests
- `SubprocessSshClient` — production implementation; used by S04 real dispatch path
- Localhost SSH integration test — proves the subprocess approach works before S02 builds on it

## Open Questions

- **`StrictHostKeyChecking` default**: using `accept-new` (accept unknown hosts, reject changed keys) rather than `no` (accept all). This is safer for first-time connections to new workers. If operators prefer `no` for fully automated environments, they can override via `~/.ssh/config`. Current thinking: `accept-new` is the right default — revisable if it causes friction.
- **SSH session lifetime**: S01's `SshClient` trait connects per-call (no persistent session). S02 will call connect + exec in sequence. This is fine for sequential manifest-deliver + exec but means two TCP connections per job. Connection multiplexing (`ControlMaster`) is out of scope for M008 but the trait boundary keeps the option open.
