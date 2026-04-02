---
id: T01
parent: S01
milestone: M008
provides:
  - WorkerConfig struct with host, user, key_env, port (default 22), deny_unknown_fields
  - ServerConfig extended with workers: Vec<WorkerConfig> (default empty) and ssh_timeout_secs: u64 (default 3)
  - ServerConfig::validate() extended to collect all worker errors (D018 pattern)
  - 6 new test_worker_config_* tests covering roundtrip, defaults, deny_unknown_fields, validation
  - Commented [[workers]] example block in examples/server.toml
key_files:
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/tests.rs
  - examples/server.toml
key_decisions:
  - WorkerConfig uses #[serde(deny_unknown_fields)] per D017 — unknown fields in [[workers]] produce parse errors
  - key_env stores env var name only, never the key value (D112) — documented in struct comment
  - Worker validation collects all errors before returning (D018) — error message lists each worker[i] violation
  - dead_code annotations on key_env, port, ssh_timeout_secs since T02 (SshClient) hasn't consumed them yet
patterns_established:
  - default_ssh_port() / default_workers() / default_ssh_timeout_secs() — same pattern as existing default_* fns
  - Worker validation loop: iterate enumerate, push errors, bail with joined message
observability_surfaces:
  - ServerConfig::load() returns descriptive error including worker index and field name for diagnosis
  - validate() error: "invalid worker configuration:\n  worker[0]: host must not be empty"
duration: 15min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T01: Add WorkerConfig and ssh_timeout_secs to ServerConfig

**`WorkerConfig` struct + `ServerConfig` extended with `workers`/`ssh_timeout_secs`, 6 new tests all green, zero workspace failures**

## What Happened

Added `WorkerConfig` above `ServerConfig` in `config.rs` with four fields (`host`, `user`, `key_env`, `port`) and `#[serde(deny_unknown_fields)]` per D017. Added three default functions: `default_workers()`, `default_ssh_timeout_secs()`, `default_ssh_port()` — consistent with the existing `default_host()` / `default_port()` pattern.

Extended `ServerConfig` with `workers: Vec<WorkerConfig>` (default empty) and `ssh_timeout_secs: u64` (default 3), both wrapped in `#[serde(default = "...")]` so existing `server.toml` files without `[[workers]]` parse without error.

Extended `validate()` to iterate workers, collect all validation errors (per D018 — no fail-fast), and return a combined error message listing each `worker[i]` violation. Added `#[allow(dead_code)]` annotations on `key_env`, `port`, and `ssh_timeout_secs` since T02 (`SshClient`) hasn't consumed them yet; annotated comments reference T02 explicitly.

Added 6 new tests in `tests.rs` covering the full matrix: roundtrip parse, port default (22), no-workers-default parse, deny_unknown_fields rejection, empty-host validation failure, empty-user validation failure. Appended a commented `[[workers]]` example block to `examples/server.toml`.

## Verification

- `cargo test -p smelt-cli test_worker_config` — 5 of 6 (filter matched 5); `test_server_config_no_workers_parses` matched via `test_server_config` filter: all pass
- `cargo test -p smelt-cli test_server_config` — 4 tests (3 existing + `test_server_config_no_workers_parses`): all pass
- `cargo test --workspace` — 155 tests, 0 failures

## Diagnostics

- `ServerConfig::load()` error: includes file path, TOML parse error, or validation message
- Validation error format: `"invalid worker configuration:\n  worker[0]: host must not be empty"`
- Worker index and field name always included — operator can locate the offending `[[workers]]` entry in `server.toml` without re-reading the file

## Deviations

None — implementation matched the plan exactly.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/config.rs` — `WorkerConfig` struct, default functions, `ServerConfig` extensions, extended `validate()`
- `crates/smelt-cli/src/serve/tests.rs` — 6 new `test_worker_config_*` tests
- `examples/server.toml` — commented `[[workers]]` example block appended
