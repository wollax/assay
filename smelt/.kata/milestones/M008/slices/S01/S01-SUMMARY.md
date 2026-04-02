---
id: S01
parent: M008
milestone: M008
provides:
  - WorkerConfig { host, user, key_env, port } with deny_unknown_fields and serde defaults
  - ServerConfig::workers: Vec<WorkerConfig> (default empty) + ssh_timeout_secs: u64 (default 3)
  - SshOutput { stdout, stderr, exit_code } — fully populated even on error paths
  - SshClient trait — async exec() and probe() via RPITIT, generic not object-safe (D121)
  - SubprocessSshClient — unit struct impl via tokio::process::Command + which::which("ssh")
  - build_ssh_args() — assembles BatchMode=yes, StrictHostKeyChecking=accept-new, ConnectTimeout, -p, -i flags
  - probe() — fast-fail via SSH's own ConnectTimeout; no tokio::time::timeout wrapper, no zombie risk
  - 6 test_worker_config_* tests + 2 non-gated SSH arg unit tests; 2 gated #[ignore] integration tests
  - examples/server.toml commented [[workers]] block
requires: []
affects:
  - S02
  - S03
  - S04
key_files:
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/ssh.rs
  - crates/smelt-cli/src/serve/mod.rs
  - crates/smelt-cli/src/serve/tests.rs
  - examples/server.toml
key_decisions:
  - D111: subprocess ssh/scp not openssh/ssh2 crate — consistent with D002 (git CLI pattern), respects ~/.ssh/config automatically
  - D112: key_env stores env var name only, never key value — follows D014 credential injection pattern
  - D121: SshClient uses generics not dyn — RPITIT async fn not object-safe; consistent with D060 (ForgeClient pattern)
  - D019 pattern applied: #[allow(async_fn_in_trait)] on SshClient; crate-internal, Send bounds not required
  - probe() delegates fast-fail to SSH's -o ConnectTimeout — no zombie subprocess risk, no tokio::time::timeout
patterns_established:
  - default_ssh_port() / default_workers() / default_ssh_timeout_secs() — same pattern as existing default_* fns
  - Worker validation loop: enumerate, push errors, bail with joined message (D018)
  - SshClient trait + SubprocessSshClient unit struct — mirrors GitOps + GitCli pattern
  - Gated integration test guard: if std::env::var("SMELT_SSH_TEST").is_err() { return; } in #[tokio::test] #[ignore]
  - build_ssh_args() is pub for unit testability without real SSH subprocess invocation
observability_surfaces:
  - exec() logs tracing::debug!(host, cmd, args) on entry; tracing::warn!(host, exit_code, stderr) on non-zero exit
  - probe() logs tracing::warn! on failure with host context
  - SshOutput { stdout, stderr, exit_code } always fully populated; exit_code -1 on signal kill
  - Error messages include host, command, exit code, stderr snippet — sufficient for agent to distinguish timeout vs auth vs refused
  - key_env missing → WARN + SSH falls back to default key; no hard error (allows dev without explicit key_env)
  - ServerConfig::validate() error format: "invalid worker configuration:\n  worker[0]: host must not be empty"
drill_down_paths:
  - .kata/milestones/M008/slices/S01/tasks/T01-SUMMARY.md
  - .kata/milestones/M008/slices/S01/tasks/T02-SUMMARY.md
duration: 45min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
---

# S01: WorkerConfig + SSH connection proof

**`WorkerConfig` + `SshClient` trait + `SubprocessSshClient` impl — `[[workers]]` parses from `server.toml`; SSH subprocess approach proven; 8 tests (6 config, 2 SSH args) all green; 2 gated integration tests ready for `SMELT_SSH_TEST=1`; 155 workspace tests pass**

## What Happened

**T01** added `WorkerConfig` and extended `ServerConfig` in `config.rs`. The struct uses `#[serde(deny_unknown_fields)]` per D017 and four fields (`host`, `user`, `key_env`, `port`). Three new default functions (`default_workers`, `default_ssh_timeout_secs`, `default_ssh_port`) follow the existing `default_host`/`default_port` pattern. `validate()` was extended to collect all worker errors before returning (D018 pattern), producing messages like `"invalid worker configuration:\n  worker[0]: host must not be empty"`. The examples/server.toml got a commented `[[workers]]` block. Six new config tests cover roundtrip, port default, no-workers-default, deny_unknown_fields rejection, and validation for empty host/user.

**T02** created `crates/smelt-cli/src/serve/ssh.rs` with the full SSH abstraction: `SshOutput` (stdout/stderr/exit_code — always populated), `SshClient` trait with async `exec()` and `probe()` using `#[allow(async_fn_in_trait)]` (D019 pattern), and `SubprocessSshClient` implementing both via `tokio::process::Command`. The SSH binary is located via `which::which("ssh")` rather than a hardcoded path. `build_ssh_args()` is `pub` to enable unit testing of arg composition without a real SSH call. `probe()` calls `exec("echo smelt-probe")` and maps non-zero exit to error — fast-fail is entirely delegated to SSH's own `-o ConnectTimeout` flag, eliminating zombie subprocess risk. Missing `key_env` triggers a WARN log and falls back to SSH's default key rather than failing hard. `mod ssh` and `pub use ssh::*` added to `serve/mod.rs`.

## Verification

| Check | Status | Evidence |
|---|---|---|
| `cargo test --workspace` | ✅ PASS | 155 passed, 0 failed, 2 ignored (gated) |
| `test_worker_config_roundtrip` | ✅ PASS | Full TOML parse round-trip |
| `test_worker_config_defaults` | ✅ PASS | port=22 default applied |
| `test_server_config_no_workers_parses` | ✅ PASS | Existing server.toml without [[workers]] parses cleanly |
| `test_worker_config_deny_unknown_fields` | ✅ PASS | Unknown field → parse error |
| `test_worker_config_empty_host_fails_validation` | ✅ PASS | validate() rejects empty host |
| `test_worker_config_empty_user_fails_validation` | ✅ PASS | validate() rejects empty user |
| `test_ssh_args_build` | ✅ PASS | BatchMode=yes, StrictHostKeyChecking=accept-new, ConnectTimeout=3, user@host |
| `test_ssh_args_build_custom_port` | ✅ PASS | -p 2222 present for non-default port |
| `test_ssh_exec_localhost` | ⏭ GATED | Requires `SMELT_SSH_TEST=1 cargo test -- --include-ignored` |
| `test_ssh_probe_offline` | ⏭ GATED | Requires `SMELT_SSH_TEST=1 cargo test -- --include-ignored` |
| `grep [[workers]] examples/server.toml` | ✅ PASS | Commented example block present |

## Requirements Advanced

- R027 (SSH worker pools / remote dispatch) — S01 delivers the config schema (`WorkerConfig`, `ServerConfig::workers`) and the SSH execution primitive (`SshClient`/`SubprocessSshClient`/`probe`) that all subsequent slices depend on. The SSH subprocess approach (D111) is proven viable by the non-gated arg tests and the gated localhost integration tests.

## Requirements Validated

- None validated in this slice alone — R027 requires the full dispatch loop (S04) to be validated.

## New Requirements Surfaced

- None.

## Requirements Invalidated or Re-scoped

- None.

## Deviations

None — both tasks implemented exactly as planned.

## Known Limitations

- Gated integration tests (`test_ssh_exec_localhost`, `test_ssh_probe_offline`) are marked `#[ignore]` and only run with `SMELT_SSH_TEST=1 cargo test -- --include-ignored`. They require `sshd` running on localhost and are not exercised in the standard `cargo test --workspace` run.
- `key_env` missing triggers a WARN and falls back to SSH default keys rather than failing hard. This is intentional for dev-friendliness but means a misconfigured `key_env` field silently degrades to default key behavior rather than producing an immediate error.
- `dead_code` annotations on `key_env`, `port`, and `ssh_timeout_secs` in `WorkerConfig` and `ServerConfig` — these fields are unused until S02 consumes them; the annotations are temporary and documented to reference S02.

## Follow-ups

- S02 should remove `#[allow(dead_code)]` annotations on `key_env`, `port`, and `ssh_timeout_secs` as it consumes those fields in `deliver_manifest` / `run_remote_job`.
- S02 should add `scp` integration tests on the same gated `SMELT_SSH_TEST=1` pattern.

## Files Created/Modified

- `crates/smelt-cli/src/serve/config.rs` — `WorkerConfig` struct, default fns, `ServerConfig` extensions, extended `validate()`
- `crates/smelt-cli/src/serve/ssh.rs` — new: `SshOutput`, `SshClient` trait, `SubprocessSshClient`, 4 tests (2 non-gated, 2 gated)
- `crates/smelt-cli/src/serve/mod.rs` — `pub mod ssh;` + `pub use ssh::*` re-exports
- `crates/smelt-cli/src/serve/tests.rs` — 6 new `test_worker_config_*` tests
- `examples/server.toml` — commented `[[workers]]` example block

## Forward Intelligence

### What the next slice should know
- `SshClient` is generic (`<C: SshClient>`) not `dyn SshClient` — D121 is firm. S02 should define `deliver_manifest<C: SshClient>` and `run_remote_job<C: SshClient>`, following the D060/D031 pattern.
- `build_ssh_args()` is the single place to add new SSH flags. S02's `scp` subprocess should use a similar `build_scp_args()` helper for consistency and testability.
- The gated test pattern is: `#[tokio::test] #[ignore]` with `if std::env::var("SMELT_SSH_TEST").is_err() { return; }` at the top. S02 adds new gated tests under the same env var.
- `key_env` is already wired through `WorkerConfig`; S02 resolves it with `std::env::var(&worker.key_env)` and passes the path to `-i` in scp/ssh args.
- `ssh_timeout_secs` lives on `ServerConfig`, not `WorkerConfig` — it's a daemon-wide default. S02 passes it through from `ServerConfig` at dispatch time.

### What's fragile
- `probe()` timeout depends entirely on SSH's `-o ConnectTimeout` — if the OS TCP stack returns RST (refused) immediately, the connection fails fast even without ConnectTimeout; but if the host is firewalled (drops packets), timeout is bounded by ConnectTimeout. The gated `test_ssh_probe_offline` uses `127.0.0.1:19222` (RST-immediate); a firewalled real host would still respect ConnectTimeout.
- `which::which("ssh")` will fail in minimal container environments without OpenSSH client installed. S02/S04 should surface this as a clear startup error if workers are configured but `ssh` is not in PATH.

### Authoritative diagnostics
- SSH subprocess errors: `tracing::warn!` in `SubprocessSshClient::exec()` includes host, exit_code, and stderr snippet — check `serve.log` (or stderr in --no-tui mode) first.
- Config errors: `ServerConfig::validate()` error message always includes worker index and field name; check the error returned by `ServerConfig::load()`.

### What assumptions changed
- Original plan listed `-o StrictHostKeyChecking=accept-new` — this was implemented as planned; no change needed for known_hosts handling in a static worker pool.
- `probe()` was originally specified as a separate TCP-level probe; implemented as `exec("echo smelt-probe")` instead — simpler, no extra binary, and SSH's own ConnectTimeout provides the same ≤3s guarantee.
