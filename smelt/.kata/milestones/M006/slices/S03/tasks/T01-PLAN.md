---
estimated_steps: 5
estimated_files: 4
---

# T01: ServerConfig TOML struct + examples/server.toml

**Slice:** S03 — Ratatui TUI + Server Config + Graceful Shutdown
**Milestone:** M006

## Description

Create the `ServerConfig` TOML struct with all daemon-wide configuration fields, a `load()` function with validation, and ship `examples/server.toml` as the canonical documented config. This is a pure data/parsing task with no runtime dependencies — independently verifiable before any other S03 work.

## Steps

1. Create `crates/smelt-cli/src/serve/config.rs`:
   - `ServerNetworkConfig { host: String, port: u16 }` with serde defaults (`default_host = "127.0.0.1"`, `default_port = 8765`)
   - `ServerConfig { queue_dir: PathBuf, max_concurrent: usize, retry_attempts: u32, retry_backoff_secs: u64, server: ServerNetworkConfig }` with `#[serde(deny_unknown_fields)]` and serde defaults for optional fields (retry_attempts=3, retry_backoff_secs=5, `#[serde(default)]` on `server`)
   - `impl ServerConfig { pub fn load(path: &std::path::Path) -> anyhow::Result<ServerConfig> }` — reads file to string, calls `toml::from_str`, then validates: `max_concurrent > 0` (error: "max_concurrent must be at least 1"), `server.port > 0` (error: "server.port must be non-zero")

2. Register the new module in `crates/smelt-cli/src/serve/mod.rs`:
   - Add `pub(crate) mod config;` and `pub(crate) use config::ServerConfig;`

3. Write `examples/server.toml` with inline `#` comments explaining every field:
   ```toml
   # Directory to watch for incoming manifest .toml files
   queue_dir = "/tmp/smelt-queue"
   # Maximum number of jobs to run concurrently
   max_concurrent = 2
   # Maximum retry attempts before marking a job as permanently failed
   retry_attempts = 3
   # Seconds to wait between retry attempts
   retry_backoff_secs = 5

   [server]
   # Host address for the HTTP API
   host = "127.0.0.1"
   # Port for the HTTP API
   port = 8765
   ```

4. Add 3 unit tests to `crates/smelt-cli/src/serve/tests.rs`:
   - `test_server_config_roundtrip`: parse a complete TOML string with all fields; assert each field value
   - `test_server_config_missing_queue_dir`: parse TOML without `queue_dir`; assert `toml::from_str` returns error (serde will error since it's required)
   - `test_server_config_invalid_max_concurrent`: parse TOML with `max_concurrent = 0`; call `load()` (or equivalent validate step); assert error message contains "max_concurrent"

5. Run `cargo test -p smelt-cli serve::tests::test_server_config` and fix any issues; run `cargo build -p smelt-cli` to confirm clean build.

## Must-Haves

- [ ] `ServerConfig::load("examples/server.toml")` returns `Ok(config)` with correct field values
- [ ] `max_concurrent = 0` returns an `Err` containing "max_concurrent" in the message
- [ ] Missing required field `queue_dir` returns a parse error
- [ ] `#[serde(deny_unknown_fields)]` is present — unknown fields in TOML cause a parse error (verified by adding a test key and expecting error)
- [ ] `examples/server.toml` file exists and has inline comments for all fields
- [ ] 3 new tests pass: `cargo test -p smelt-cli serve::tests::test_server_config` → 3 passed

## Verification

- `cargo test -p smelt-cli serve::tests::test_server_config -- --nocapture` → 3 tests pass
- `cargo build -p smelt-cli 2>&1 | grep "^error"` → no errors
- `grep -c "#" examples/server.toml` → at least 7 (one comment per field)

## Observability Impact

- Signals added/changed: `ServerConfig::load()` emits an `anyhow::Error` on bad config — the error message reaches the user before any component starts (fail-fast before daemon launches)
- How a future agent inspects this: `smelt serve --config bad.toml` exits immediately with a descriptive error; no background processes started
- Failure state exposed: parse/validation error printed to stderr with full anyhow chain; `examples/server.toml` documents the expected shape

## Inputs

- `crates/smelt-cli/src/serve/mod.rs` — module registry; add `pub(crate) mod config;`
- `crates/smelt-cli/src/serve/tests.rs` — existing test file; append new tests
- `crates/smelt-cli/Cargo.toml` — already has `toml.workspace = true` (no new deps needed)
- `examples/` directory — already exists with other example TOMLs

## Expected Output

- `crates/smelt-cli/src/serve/config.rs` — new file with `ServerConfig` + `ServerNetworkConfig` + `load()` + serde defaults
- `crates/smelt-cli/src/serve/mod.rs` — updated with `pub(crate) mod config;` + `pub(crate) use config::ServerConfig;`
- `examples/server.toml` — new file; canonical documented server configuration
- `crates/smelt-cli/src/serve/tests.rs` — 3 new config tests appended
