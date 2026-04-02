---
id: T01
parent: S01
milestone: M010
provides:
  - AuthConfig struct with deny_unknown_fields for TOML parsing
  - ServerConfig.auth optional field (backward-compatible)
  - ResolvedAuth struct with resolved token values
  - resolve_auth() fails fast on missing/empty env vars
  - auth_middleware() enforces bearer token with read/write split
  - build_router() accepts Option<ResolvedAuth> and applies middleware layer
  - start_test_server_with_auth() helper for auth-enabled test servers
key_files:
  - crates/smelt-cli/src/serve/config.rs
  - crates/smelt-cli/src/serve/http_api.rs
  - crates/smelt-cli/src/commands/serve.rs
  - crates/smelt-cli/src/serve/tests/mod.rs
key_decisions:
  - "auth_middleware receives Option<ResolvedAuth> as state; always applied but no-ops when None"
  - "read_env_var helper rejects both missing and empty env vars with descriptive messages"
  - "middleware layer applied before with_state(state) so it wraps all routes"
patterns_established:
  - "Bearer token extraction pattern: parse Authorization header, strip 'Bearer ' prefix"
  - "Permission check pattern: GET/HEAD = read (accept read OR write token), else = write (accept only write token)"
  - "start_test_server_with_auth() test helper for auth-enabled integration tests"
observability_surfaces:
  - "tracing::warn on 401/403 with method + path"
  - "tracing::info at startup listing configured auth env var names"
  - "JSON error bodies on 401 and 403 with reason strings"
  - "Startup abort with descriptive error naming missing/empty env var"
duration: 10min
verification_result: passed
completed_at: 2026-03-24T00:00:00Z
blocker_discovered: false
---

# T01: AuthConfig struct, env var resolution, and auth middleware function

**Bearer token auth infrastructure: config parsing, env var resolution, and read/write permission middleware for the HTTP API**

## What Happened

Added `AuthConfig` struct to `config.rs` with `write_token_env: String` and `read_token_env: Option<String>`, both with `deny_unknown_fields`. Added `auth: Option<AuthConfig>` with `#[serde(default)]` to `ServerConfig` so existing TOML files without `[auth]` continue to parse.

In `http_api.rs`, added `ResolvedAuth` (Clone, Debug) holding resolved token values, `resolve_auth()` that reads env vars by name and rejects empty/missing with clear error messages, and `auth_middleware()` that enforces the read/write split: GET/HEAD accept read_token OR write_token, all other methods require write_token only. Returns 401 JSON for missing/malformed auth, 403 JSON for insufficient permissions.

Updated `build_router()` to accept `Option<ResolvedAuth>` and apply the middleware via `axum::middleware::from_fn_with_state`. Updated `commands/serve.rs` to resolve auth config at startup with info logging. Updated test helper to forward `None` auth by default with a new `start_test_server_with_auth()` variant for T02's auth tests.

## Verification

- `cargo check --workspace` — clean, zero errors
- `cargo doc --workspace --no-deps` — zero warnings
- `cargo test --package smelt-cli --lib serve` — all 54 tests pass
- All existing tests pass without auth (backward compat confirmed)
- One pre-existing failing test (`test_cli_run_invalid_manifest`) is unrelated to these changes

## Diagnostics

- 401 responses include `{"error": "missing or malformed Authorization: Bearer <token> header"}`
- 403 responses include `{"error": "token does not have {read|write} permission"}`
- `tracing::warn!` on rejected requests logs method + path (never token values)
- `tracing::info!` at startup when auth enabled logs env var names (never values)
- Startup fails fast with message naming the specific env var that is missing or empty

## Deviations

None.

## Known Issues

None.

## Files Created/Modified

- `crates/smelt-cli/src/serve/config.rs` — Added `AuthConfig` struct and `ServerConfig.auth` field
- `crates/smelt-cli/src/serve/http_api.rs` — Added `ResolvedAuth`, `resolve_auth()`, `auth_middleware()`, updated `build_router()` signature
- `crates/smelt-cli/src/commands/serve.rs` — Auth resolution at startup with tracing
- `crates/smelt-cli/src/serve/tests/mod.rs` — Added `start_test_server_with_auth()` helper
