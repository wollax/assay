---
id: T02
parent: S01
milestone: M010
provides:
  - execute() resolves auth env vars and passes Option<ResolvedAuth> to build_router()
  - start_test_server() backward-compat wrapper delegates to start_test_server_with_auth(state, None)
  - start_test_server_with_auth() accepts Option<ResolvedAuth> for auth-enabled test scenarios
  - tracing::info! at startup logs env var names when auth is enabled (never values)
  - ResolvedAuth and resolve_auth accessible from crate::serve::http_api for serve command and tests
key_files:
  - crates/smelt-cli/src/commands/serve.rs
  - crates/smelt-cli/src/serve/tests/mod.rs
  - crates/smelt-cli/src/serve/mod.rs
key_decisions:
  - "T01 already completed all T02 wiring — no additional code changes needed"
patterns_established:
  - "Test helper layering: start_test_server() wraps start_test_server_with_auth(state, None) for backward compat"
observability_surfaces:
  - "tracing::info! with write_token_env and read_token_env fields at startup when auth enabled"
  - "resolve_auth() fails fast with error naming the specific env var that is missing or empty"
duration: 5min
verification_result: passed
completed_at: 2026-03-24T12:00:00Z
blocker_discovered: false
---

# T02: Wire auth into serve startup and update test helper

**Auth wiring into execute() and test helper already complete from T01 — verified all 286 tests pass with zero failures**

## What Happened

T01 proactively completed all T02 wiring during its implementation. The `execute()` function in `serve.rs` already resolves auth via `resolve_auth()` after config load, logs env var names when auth is enabled, and passes `resolved_auth` to `build_router()`. The test helper in `tests/mod.rs` already has `start_test_server_with_auth()` accepting `Option<ResolvedAuth>` and `start_test_server()` wrapping it with `None` for backward compatibility. The `http_api` module already exports `ResolvedAuth`, `resolve_auth`, and `build_router` correctly.

This task validated that all wiring is correct and all existing tests pass unchanged.

## Verification

- `cargo test --workspace`: 286 tests passed, 0 failed (81+3+23+16+5+155+3 across all crates + doctests)
- `test_serve_http_responds_while_running`: passed (uses config with no `[auth]` section)
- `cargo clippy --workspace`: clean, no warnings
- All 5 must-haves verified:
  - execute() resolves auth and passes to build_router ✓
  - start_test_server() accepts Option<ResolvedAuth> ✓
  - Existing callsites pass None ✓
  - All 286+ tests pass ✓
  - test_serve_http_responds_while_running passes ✓

### Slice-Level Verification (intermediate — partial expected)

| Check | Status | Notes |
|-------|--------|-------|
| `cargo test --workspace` 0 failures | ✓ PASS | 286 tests, 0 failures |
| `cargo clippy --workspace` clean | ✓ PASS | No warnings |
| Auth integration tests (T03 scope) | ⏳ PENDING | T03 will add these |
| `cargo doc --workspace --no-deps` | ⏳ NOT RUN | Final check in T03 |

## Diagnostics

- Auth startup logging: when `[auth]` section present, `tracing::info!` emits `write_token_env` and `read_token_env` field values (env var names, never token values)
- Startup failure: `resolve_auth()` returns error with specific env var name if unset or empty, before server binds

## Deviations

No code changes were needed — T01 already completed all wiring that T02 specified. This task was purely verification.

## Known Issues

None.

## Files Created/Modified

No files modified in this task — all changes were made in T01. Files verified:
- `crates/smelt-cli/src/commands/serve.rs` — execute() with auth resolution and startup logging
- `crates/smelt-cli/src/serve/tests/mod.rs` — start_test_server() and start_test_server_with_auth()
- `crates/smelt-cli/src/serve/mod.rs` — re-exports for http_api module
