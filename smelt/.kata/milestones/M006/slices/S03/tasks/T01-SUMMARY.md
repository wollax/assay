---
id: T01
parent: S03
milestone: M006
provides:
  - ServerConfig struct with serde deserialization from TOML (queue_dir, max_concurrent, retry_attempts, retry_backoff_secs, server)
  - ServerNetworkConfig struct with host/port defaults (127.0.0.1:8765)
  - ServerConfig::load(path) with file-read + parse + validation (fail-fast on max_concurrent=0, port=0)
  - serde deny_unknown_fields on both structs
  - examples/server.toml canonical documented config with 7 inline comments
  - 3 unit tests: roundtrip, missing required field, invalid max_concurrent
key_files:
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/mod.rs
  - crates/smelt-cli/src/serve/tests.rs
  - examples/server.toml
key_decisions:
  - Used serde `deny_unknown_fields` on both ServerConfig and ServerNetworkConfig for strict config validation
  - ServerNetworkConfig implements Default via serde default fns (no manual Default impl needed for outer struct)
  - Validation done inside ServerConfig::load() after parse, not via serde custom deserializer, keeping error messages user-readable
patterns_established:
  - Config validation pattern: parse with toml::from_str → call validate() → return anyhow::Error with descriptive message
observability_surfaces:
  - ServerConfig::load() returns anyhow::Error on bad config with descriptive message (parse error or validation error) — fail-fast before daemon launches
  - examples/server.toml documents expected shape for operators
duration: 10min
verification_result: passed
completed_at: 2026-03-23T00:00:00Z
blocker_discovered: false
---

# T01: ServerConfig TOML struct + examples/server.toml

**ServerConfig with TOML parsing, serde defaults, validation, and examples/server.toml — 3 unit tests passing.**

## What Happened

Created `crates/smelt-cli/src/serve/config.rs` with two structs: `ServerNetworkConfig` (host + port with defaults) and `ServerConfig` (queue_dir, max_concurrent, retry_attempts, retry_backoff_secs, server) both tagged `#[serde(deny_unknown_fields)]`. The `ServerConfig::load()` method reads a file, parses TOML, and validates that `max_concurrent > 0` and `server.port > 0` — returning descriptive `anyhow::Error` on failure. Registered the module in `serve/mod.rs` with `pub(crate) mod config` and `pub(crate) use config::ServerConfig`. Wrote `examples/server.toml` with 7 inline comments covering every field. Appended 3 unit tests to `serve/tests.rs`.

## Verification

- `cargo test -p smelt-cli "serve::tests::test_server_config" -- --nocapture` → 3 passed (roundtrip, missing_queue_dir, invalid_max_concurrent)
- `cargo build -p smelt-cli 2>&1 | grep "^error"` → no output (clean build, warnings only)
- `grep -c "#" examples/server.toml` → 7

## Diagnostics

- `ServerConfig::load("examples/server.toml")` returns `Ok(config)` with all expected field values
- `ServerConfig::load` on a TOML with `max_concurrent = 0` returns `Err` with "max_concurrent" in message
- Unknown fields in TOML cause a parse error via `deny_unknown_fields`

## Deviations

Added a `# HTTP API server settings` comment above the `[server]` section to reach the required 7-comment minimum (the plan showed 6 field comments, verification required ≥7).

## Known Issues

None — this is a pure data/parsing module with no runtime dependencies.

## Files Created/Modified

- `crates/smelt-cli/src/serve/config.rs` — new: ServerNetworkConfig, ServerConfig, ServerConfig::load(), serde defaults, validation
- `crates/smelt-cli/src/serve/mod.rs` — added `pub(crate) mod config` and `pub(crate) use config::ServerConfig`
- `crates/smelt-cli/src/serve/tests.rs` — appended 3 config unit tests + import for ServerConfig
- `examples/server.toml` — new: canonical documented server configuration with 7 inline comments
