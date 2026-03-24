---
estimated_steps: 5
estimated_files: 3
---

# T01: Add WorkerConfig and ssh_timeout_secs to ServerConfig

**Slice:** S01 — WorkerConfig + SSH connection proof
**Milestone:** M008

## Description

Extend `ServerConfig` with `workers: Vec<WorkerConfig>` and `ssh_timeout_secs: u64` so that `[[workers]]` entries in `server.toml` parse and validate correctly. This is a pure config extension — no SSH code yet. It locks the data contract that T02 (`SshClient`) and all downstream slices (S02–S04) depend on. All existing `server.toml` files without `[[workers]]` must continue to parse without error.

Key constraints from research:
- `ServerConfig` has `#[serde(deny_unknown_fields)]` — new fields need `#[serde(default = "...")]` or existing configs break
- `WorkerConfig` must also use `#[serde(deny_unknown_fields)]` (D017)
- `port` defaults to 22 via `fn default_ssh_port() -> u16 { 22 }`
- `key_env` stores an env var *name* (D112), never the key value
- Validation must reject empty `host` or `user` following the `KubernetesConfig` pattern (collect errors, D018)

## Steps

1. Open `crates/smelt-cli/src/serve/config.rs`. Add default functions: `fn default_workers() -> Vec<WorkerConfig> { vec![] }`, `fn default_ssh_timeout_secs() -> u64 { 3 }`, `fn default_ssh_port() -> u16 { 22 }`.
2. Define `WorkerConfig` struct above `ServerConfig`:
   ```rust
   #[derive(Debug, Deserialize, Clone)]
   #[serde(deny_unknown_fields)]
   pub struct WorkerConfig {
       pub host: String,
       pub user: String,
       pub key_env: String,
       #[serde(default = "default_ssh_port")]
       pub port: u16,
   }
   ```
3. Add `#[serde(default = "default_workers")] pub workers: Vec<WorkerConfig>` and `#[serde(default = "default_ssh_timeout_secs")] pub ssh_timeout_secs: u64` to `ServerConfig`.
4. Extend `ServerConfig::validate()` to iterate over `self.workers` and collect errors for any entry where `host.trim().is_empty()` or `user.trim().is_empty()`, returning them aggregated (following D018 pattern — collect all errors, not fail-fast). Use `anyhow::bail!` with a combined message listing all worker validation errors.
5. Add tests to `crates/smelt-cli/src/serve/tests.rs`:
   - `test_worker_config_roundtrip`: full TOML with one `[[workers]]` entry, assert all fields parsed correctly
   - `test_worker_config_defaults`: `[[workers]]` entry with only `host`, `user`, `key_env` — assert `port == 22`
   - `test_server_config_no_workers_parses`: TOML without `[[workers]]` — assert `workers` is empty vec and `ssh_timeout_secs == 3`
   - `test_worker_config_deny_unknown_fields`: `[[workers]]` with an unknown field — assert parse error
   - `test_worker_config_empty_host_fails_validation`: worker entry with `host = ""` — assert `validate()` returns `Err` mentioning the host issue
   - `test_worker_config_empty_user_fails_validation`: worker entry with `user = ""` — assert error

6. Add commented `[[workers]]` block to `examples/server.toml` after the `[server]` section:
   ```toml
   # SSH worker pool — optional. When present, smelt serve dispatches jobs to these
   # remote hosts instead of running them locally. Jobs are round-robined across
   # available workers; unreachable workers are skipped and the job is re-queued.
   #
   # [[workers]]
   # host = "worker1.example.com"
   # user = "smelt"
   # key_env = "WORKER_SSH_KEY"   # name of env var holding path to SSH private key
   # port = 22                    # optional, default 22
   ```

## Must-Haves

- [ ] `WorkerConfig` struct compiles with `#[derive(Deserialize)]` and `#[serde(deny_unknown_fields)]`
- [ ] `ServerConfig` gains `workers: Vec<WorkerConfig>` (default empty) and `ssh_timeout_secs: u64` (default 3) — both with `#[serde(default = "...")]`
- [ ] Existing `server.toml` without `[[workers]]` parses without error (proven by `test_server_config_no_workers_parses` and `test_server_config_roundtrip`)
- [ ] `[[workers]]` entry with unknown field produces a parse error (proven by `test_worker_config_deny_unknown_fields`)
- [ ] Worker entry with empty `host` or `user` fails `validate()` with an informative error
- [ ] `WorkerConfig::port` defaults to 22 when omitted
- [ ] `examples/server.toml` has a commented `[[workers]]` block
- [ ] `cargo test -p smelt-cli test_worker_config` — all 6 new tests pass
- [ ] `cargo test -p smelt-cli test_server_config` — existing tests still pass

## Verification

- `cargo test -p smelt-cli test_worker_config` — all 6 new config tests pass
- `cargo test -p smelt-cli test_server_config` — existing 3 config tests still pass
- `cargo test --workspace` — zero failures

## Observability Impact

- Signals added/changed: None at runtime — this task is purely config parsing. Validation errors include field names and worker index for diagnosis.
- How a future agent inspects this: `ServerConfig::load()` returns a descriptive error string; worker validation errors include the worker index and field name so operators can locate the offending entry in `server.toml`
- Failure state exposed: Validation error message lists all invalid worker entries before returning, consistent with D018 (collect-all-errors)

## Inputs

- `crates/smelt-cli/src/serve/config.rs` — existing `ServerConfig` with `deny_unknown_fields`, `ServerNetworkConfig`, and default function pattern
- `crates/smelt-core/src/manifest.rs::KubernetesConfig` — pattern for `key_env: String` field and validation
- `crates/smelt-core/src/forge.rs::ForgeConfig` — `token_env: String` pattern
- `examples/server.toml` — existing file to append commented block to

## Expected Output

- `crates/smelt-cli/src/serve/config.rs` — `WorkerConfig` struct + extended `ServerConfig` with 2 new fields + extended `validate()`
- `crates/smelt-cli/src/serve/tests.rs` — 6 new `test_worker_config_*` tests
- `examples/server.toml` — commented `[[workers]]` block appended
