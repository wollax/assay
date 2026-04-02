# S01: WorkerConfig + SSH connection proof — Research

**Date:** 2026-03-23
**Domain:** SSH subprocess execution, config parsing, async process management
**Confidence:** HIGH

## Summary

S01 is entirely greenfield for SSH — there is no existing SSH code in the smelt codebase to reuse or adapt. The subprocess pattern for `ssh`/`scp` is well-established in this project (`git/cli.rs` shells out to `git`, `compose.rs` shells out to `docker`), so the implementation approach is unambiguous. Decision D111 settles the library choice (subprocess, not `openssh`/`ssh2` crates).

The two main deliverables are: (1) adding `[[workers]]` to `ServerConfig` — a pure config extension with a clear precedent from `KubernetesConfig`; and (2) an `SshClient` trait + `SubprocessSshClient` implementation that wraps `tokio::process::Command::new("ssh")` with a connection probe that times out within `ssh_timeout_secs`. The offline-worker fast-fail is best enforced via SSH's own `-o ConnectTimeout=N` option rather than wrapping the process in `tokio::time::timeout` — SSH's built-in timeout is more precise and handles the TCP handshake correctly.

The main risk is localhost SSH test setup: the test machine must have `sshd` running and the test user must have a valid key pair in `~/.ssh/`. The `SMELT_SSH_TEST=1` gate keeps `cargo test --workspace` green when these aren't present.

## Recommendation

Implement `SshClient` as a trait in a new file `crates/smelt-cli/src/serve/ssh.rs`. The trait has two methods:
- `fn exec(&self, worker: &WorkerConfig, cmd: &str) -> impl Future<Output = Result<SshOutput>>`
- `fn probe(&self, worker: &WorkerConfig) -> impl Future<Output = Result<()>>` — used at startup health-check and dispatch

`SubprocessSshClient` implements this trait by shelling out to `ssh`. The offline-fast-fail is proven by the `-o ConnectTimeout=<ssh_timeout_secs>` flag: SSH itself returns a non-zero exit code within the specified seconds when the host refuses or times out.

Use `tokio::process::Command` (not `std::process::Command`) to keep the subprocess non-blocking — dispatch loop runs in async context and must not block the tokio thread pool.

## Don't Hand-Roll

| Problem | Existing Solution | Why Use It |
|---------|------------------|------------|
| SSH connection timeout | `-o ConnectTimeout=N` SSH flag | SSH handles the TCP handshake + auth timeout natively; more reliable than wrapping `output().await` in `tokio::time::timeout` (tokio timeout would leave the subprocess running) |
| Preventing interactive prompts that hang | `-o BatchMode=yes` SSH flag | Disables all interactive prompts (password, host key confirmation); subprocess will exit immediately with error instead of hanging |
| Unknown-host first-connect | `-o StrictHostKeyChecking=accept-new` | Accepts new hosts (first connection to a worker), rejects changed keys (security); safer than `no` |
| Config deserialization | `#[serde(default)]` on `workers` field | Makes `[[workers]]` section optional — existing `server.toml` files without workers parse correctly; consistent with `#[serde(default)]` on `server: ServerNetworkConfig` |

## Existing Code and Patterns

- `crates/smelt-cli/src/serve/config.rs` — `ServerConfig` with `#[serde(deny_unknown_fields)]`; must add `workers: Vec<WorkerConfig>` **and** `ssh_timeout_secs: u64` as new fields; both need default functions (`default_workers()` returning `vec![]`, `default_ssh_timeout_secs()` returning `3`); `#[serde(default = "default_workers")]` makes them optional in TOML
- `crates/smelt-core/src/manifest.rs::KubernetesConfig` — `ssh_key_env: String` field is the exact parallel for `WorkerConfig::key_env`; the validation pattern (`if k.ssh_key_env.trim().is_empty() { errors.push(...) }`) applies to `WorkerConfig::host` and `WorkerConfig::user` too
- `crates/smelt-core/src/git/cli.rs` — `run_in()` method: `tokio::process::Command::new(&binary).args(...).current_dir(...).output().await` is the canonical subprocess pattern to replicate for SSH
- `crates/smelt-core/src/compose.rs` — `Command::new("docker").arg(...).output().await` — same async subprocess invocation pattern; also shows how to propagate `!output.status.success()` as an error
- `crates/smelt-cli/src/serve/tests.rs` — `SMELT_K8S_TEST=1` equivalent is `SMELT_SSH_TEST=1`; existing test infra shows `#[tokio::test]` + `#[ignore]` + env var guard pattern
- `crates/smelt-core/src/forge.rs::ForgeConfig` — `token_env: String` field is the pattern for `key_env: String`; never store the value, store the env var name

## Constraints

- `ServerConfig` has `#[serde(deny_unknown_fields)]` — any new field added must have a `#[serde(default = "...")]` annotation or it will break existing `server.toml` files that don't have `workers` or `ssh_timeout_secs`
- `WorkerConfig` must also use `#[serde(deny_unknown_fields)]` (D017) — strict parsing consistent with all other config structs
- `port` in `WorkerConfig` must default to 22 — use `fn default_ssh_port() -> u16 { 22 }` + `#[serde(default = "default_ssh_port")]`
- SSH subprocess must use `tokio::process::Command` not `std::process::Command` — dispatch loop is async; blocking the thread pool on `.output()` without tokio wrapping violates the `smelt serve` async model
- `SshClient` trait must be `Send + Sync` so it can be stored in `Arc<dyn SshClient>` for sharing across the dispatch loop — or alternatively make `SubprocessSshClient` a unit struct and pass it by clone

## Common Pitfalls

- **Adding fields to `ServerConfig` without defaults breaks existing configs** — `deny_unknown_fields` means TOML keys not in the struct error; the reverse (new struct fields not in TOML) only errors if there's no `#[serde(default)]`. Both `workers` and `ssh_timeout_secs` need defaults.
- **`std::process::Command` blocks the tokio thread pool** — use `tokio::process::Command` everywhere in async contexts; the git/compose code in smelt-core mixes both, but serve code should use tokio only.
- **SSH hangs forever without `BatchMode=yes`** — an SSH connection to a new host without known_hosts entry will pause waiting for user input; `BatchMode=yes` forces an immediate error exit instead. Always include this flag.
- **`tokio::time::timeout` does not kill the subprocess** — if you wrap `ssh_command.output().await` in `tokio::time::timeout`, the future is dropped but the SSH subprocess keeps running. Use `-o ConnectTimeout=N` instead so SSH kills itself.
- **Testing with localhost SSH** — `sshd` must be enabled on macOS (`System Settings > General > Sharing > Remote Login`) and the running user must have an authorized SSH key. The `SMELT_SSH_TEST=1` gate is critical; do not assume sshd is available.
- **`which("ssh")` for the binary path** — use `which::which("ssh")` to find the ssh binary, not a hardcoded `/usr/bin/ssh`, consistent with the `which` crate already in workspace dependencies.

## Open Risks

- **macOS `StrictHostKeyChecking=accept-new`** — available in OpenSSH 7.6+; macOS ships OpenSSH 9.x so this is safe. Linux workers also have modern OpenSSH. Not a real risk but worth noting in a comment.
- **SSH key path from env var** — `std::env::var(worker.key_env)` at dispatch time; the env var must be set in the dispatcher's environment. If absent, SSH will fall back to the default key (`~/.ssh/id_rsa`), which may or may not be correct. An explicit warning when `key_env` is set but the env var is missing prevents silent misconfiguration.
- **localhost SSH test on CI** — GitHub Actions runners do not have sshd enabled by default. `SMELT_SSH_TEST=1` must never be set in CI unless the workflow explicitly enables sshd. The test must be `#[ignore]` and guarded by the env var.
- **`SshClient` trait object safety** — if `exec()` is an `async fn` returning `impl Future`, the trait is not object-safe. Two options: (a) return `Pin<Box<dyn Future<...>>>` to get object safety, (b) use a generic `<C: SshClient>` parameter at dispatch. Option (b) is consistent with D060 (`run_watch<F: ForgeClient>`). The `SubprocessSshClient` can be a unit struct and stored directly in dispatch without `dyn`.

## Skills Discovered

| Technology | Skill | Status |
|------------|-------|--------|
| SSH subprocess | n/a (standard library pattern) | none needed |
| Rust async process | n/a (tokio::process is already in the stack) | none needed |

## Sources

- Existing codebase: `git/cli.rs` subprocess pattern (confirmed via read)
- Existing codebase: `compose.rs` tokio::process::Command pattern (confirmed via read)
- Existing codebase: `config.rs` ServerConfig deny_unknown_fields structure (confirmed via read)
- Existing codebase: `manifest.rs` KubernetesConfig ssh_key_env pattern (confirmed via grep)
- Context: D111 (subprocess SSH, not library), D112 (key_env pattern), D017 (deny_unknown_fields), D089 (SMELT_K8S_TEST=1 gating pattern)
